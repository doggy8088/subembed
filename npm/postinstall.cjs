#!/usr/bin/env node
'use strict';

const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} = require('node:fs');
const { get } = require('node:https');
const { join } = require('node:path');
const { URL } = require('node:url');

const {
  PACKAGE_NAME,
  PACKAGE_ROOT,
  artifactName,
  executableName,
  installedBinDir,
  installedBinPath,
  releaseBaseUrl,
  resolvePlatform,
} = require('./shared.cjs');

const REDIRECT_LIMIT = 5;

function packageVersion() {
  return require('../package.json').version;
}

function sha256(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function parseChecksum(checksumText) {
  const expected = checksumText.trim().split(/\s+/)[0].toLowerCase();
  if (!/^[a-f0-9]{64}$/.test(expected)) {
    throw new Error('Invalid checksum file format');
  }
  return expected;
}

function verifyChecksum(filePath, checksumText) {
  const expected = parseChecksum(checksumText);
  const actual = sha256(filePath);
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${filePath}: expected ${expected}, got ${actual}`);
  }
}

function download(url, destination, redirectsRemaining = REDIRECT_LIMIT) {
  return new Promise((resolve, reject) => {
    get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location && redirectsRemaining > 0) {
        const nextUrl = new URL(res.headers.location, url).toString();
        res.resume();
        download(nextUrl, destination, redirectsRemaining - 1).then(resolve, reject);
        return;
      }

      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(`Download failed (${res.statusCode}) for ${url}`));
        return;
      }

      const chunks = [];
      res.on('data', (chunk) => chunks.push(chunk));
      res.on('end', () => {
        writeFileSync(destination, Buffer.concat(chunks));
        resolve(destination);
      });
      res.on('error', reject);
    }).on('error', reject);
  });
}

function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8' });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const stderr = result.stderr?.trim();
    throw new Error(stderr ? `${command} failed: ${stderr}` : `Command failed: ${command} ${args.join(' ')}`);
  }
}

function extractArchive(archivePath, destination, platform = process.platform) {
  mkdirSync(destination, { recursive: true });
  if (archivePath.endsWith('.zip')) {
    if (platform !== 'win32') {
      throw new Error(`ZIP extraction is only expected on Windows, got ${platform}`);
    }
    const escapedArchivePath = archivePath.replace(/'/g, "''");
    const escapedDestination = destination.replace(/'/g, "''");
    run('powershell', [
      '-NoProfile',
      '-Command',
      `Expand-Archive -Force -Path '${escapedArchivePath}' -DestinationPath '${escapedDestination}'`,
    ]);
    return;
  }

  run('tar', ['-xJf', archivePath, '-C', destination]);
}

function findExtractedBinary(dir, binName = executableName()) {
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const entryPath = join(dir, entry.name);
    if (entry.isFile() && entry.name === binName) {
      return entryPath;
    }
  }
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const match = findExtractedBinary(join(dir, entry.name), binName);
    if (match) {
      return match;
    }
  }
  throw new Error(`Archive did not contain ${binName}`);
}

function installFromLocalBuild(platform = process.platform) {
  if (!existsSync(join(PACKAGE_ROOT, '.git'))) {
    return false;
  }

  const localRelease = join(PACKAGE_ROOT, 'target', 'release', executableName(platform));
  if (!existsSync(localRelease)) {
    return false;
  }

  const destination = installedBinPath(platform);
  mkdirSync(installedBinDir(), { recursive: true });
  copyFileSync(localRelease, destination);
  if (platform !== 'win32') {
    chmodSync(destination, 0o755);
  }
  return true;
}

async function installFromRelease(platform = process.platform, arch = process.arch) {
  const spec = resolvePlatform(platform, arch);
  const archive = artifactName(spec);
  const baseUrl = releaseBaseUrl(packageVersion());
  const binDir = installedBinDir();
  const destination = installedBinPath(platform);
  const downloadDir = join(binDir, '.download');
  const archivePath = join(downloadDir, archive);
  const checksumPath = `${archivePath}.sha256`;

  rmSync(downloadDir, { recursive: true, force: true });
  mkdirSync(downloadDir, { recursive: true });

  try {
    await download(`${baseUrl}/${archive}`, archivePath);
    await download(`${baseUrl}/${archive}.sha256`, checksumPath);
    verifyChecksum(archivePath, readFileSync(checksumPath, 'utf8'));
    extractArchive(archivePath, downloadDir, platform);

    mkdirSync(binDir, { recursive: true });
    copyFileSync(findExtractedBinary(downloadDir, executableName(platform)), destination);
    if (platform !== 'win32') {
      chmodSync(destination, 0o755);
    }
  } finally {
    rmSync(downloadDir, { recursive: true, force: true });
  }
}

async function main() {
  resolvePlatform();
  if (installFromLocalBuild()) {
    return;
  }
  await installFromRelease();
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`${PACKAGE_NAME} install failed: ${error.message}`);
    process.exit(1);
  });
}

module.exports = {
  download,
  extractArchive,
  findExtractedBinary,
  installFromLocalBuild,
  installFromRelease,
  packageVersion,
  parseChecksum,
  sha256,
  verifyChecksum,
};
