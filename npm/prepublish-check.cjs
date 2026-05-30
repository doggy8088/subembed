#!/usr/bin/env node
'use strict';

const { request } = require('node:https');
const { URL } = require('node:url');

const { SUPPORTED_TARGETS, artifactName, releaseBaseUrl } = require('./shared.cjs');

const RETRIES_ENV = 'SUBEMBED_RELEASE_ASSET_RETRIES';
const RETRY_DELAY_ENV = 'SUBEMBED_RELEASE_ASSET_RETRY_DELAY_MS';
const REDIRECT_LIMIT = 5;

function packageVersion() {
  return require('../package.json').version;
}

function expectedReleaseUrls(version = packageVersion()) {
  const baseUrl = releaseBaseUrl(version);
  return Object.values(SUPPORTED_TARGETS).flatMap((spec) => {
    const archive = artifactName(spec);
    return [`${baseUrl}/${archive}`, `${baseUrl}/${archive}.sha256`];
  });
}

function checkUrl(url, redirectsRemaining = REDIRECT_LIMIT) {
  return new Promise((resolve) => {
    const req = request(url, { method: 'HEAD' }, (res) => {
      const { statusCode = 0, headers } = res;
      res.resume();

      if (statusCode >= 300 && statusCode < 400 && headers.location && redirectsRemaining > 0) {
        const nextUrl = new URL(headers.location, url).toString();
        checkUrl(nextUrl, redirectsRemaining - 1).then(resolve);
        return;
      }

      resolve({
        url,
        ok: statusCode >= 200 && statusCode < 300,
        statusCode,
      });
    });

    req.on('error', (error) => {
      resolve({
        url,
        ok: false,
        errorMessage: error.message,
      });
    });
    req.end();
  });
}

function retryCountFromEnv() {
  return Math.max(1, Number.parseInt(process.env[RETRIES_ENV] ?? '1', 10) || 1);
}

function retryDelayMsFromEnv() {
  return Math.max(0, Number.parseInt(process.env[RETRY_DELAY_ENV] ?? '1000', 10) || 1000);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function formatFailure(result) {
  const reason = result.statusCode ? `HTTP ${result.statusCode}` : result.errorMessage;
  return `- ${result.url} (${reason})`;
}

async function verifyReleaseAssets({
  version = packageVersion(),
  check = checkUrl,
  retries = retryCountFromEnv(),
  retryDelayMs = retryDelayMsFromEnv(),
} = {}) {
  const urls = expectedReleaseUrls(version);
  let failures = [];

  for (let attempt = 1; attempt <= retries; attempt += 1) {
    const results = await Promise.all(urls.map((url) => check(url)));
    failures = results.filter((result) => !result.ok);
    if (failures.length === 0) {
      return urls;
    }
    if (attempt < retries) {
      await sleep(retryDelayMs);
    }
  }

  throw new Error(
    [
      `Missing or unavailable release assets for v${version}:`,
      ...failures.map(formatFailure),
      'Create and upload the GitHub Release assets before publishing npm.',
    ].join('\n'),
  );
}

async function main() {
  const version = packageVersion();
  const urls = await verifyReleaseAssets({ version });
  console.log(`Verified ${urls.length} release assets for v${version}.`);
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}

module.exports = {
  checkUrl,
  expectedReleaseUrls,
  formatFailure,
  packageVersion,
  retryCountFromEnv,
  retryDelayMsFromEnv,
  verifyReleaseAssets,
};
