'use strict';

const assert = require('node:assert/strict');
const { chmodSync, mkdirSync, rmSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');
const { spawnSync } = require('node:child_process');
const { after, test } = require('node:test');

const {
  BINARY_NAME,
  artifactName,
  executableName,
  installedBinDir,
  installedBinPath,
  releaseBaseUrl,
  resolvePlatform,
  supportedPlatformKeys,
} = require('../npm/shared.cjs');
const {
  findExtractedBinary,
  parseChecksum,
  sha256,
  verifyChecksum,
} = require('../npm/postinstall.cjs');
const {
  expectedReleaseUrls,
  formatFailure,
  verifyReleaseAssets,
} = require('../npm/prepublish-check.cjs');

const SANDBOX_ROOT = join(__dirname, '.sandbox');

function sandbox(name) {
  const dir = join(SANDBOX_ROOT, name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

after(() => {
  rmSync(SANDBOX_ROOT, { recursive: true, force: true });
});

test('maps supported platforms to release artifacts', () => {
  assert.deepEqual(supportedPlatformKeys(), ['darwin-arm64', 'darwin-x64', 'linux-x64', 'win32-x64']);
  assert.equal(resolvePlatform('darwin', 'arm64').rustTarget, 'aarch64-apple-darwin');
  assert.equal(resolvePlatform('darwin', 'x64').rustTarget, 'x86_64-apple-darwin');
  assert.equal(resolvePlatform('linux', 'x64').rustTarget, 'x86_64-unknown-linux-gnu');
  assert.equal(resolvePlatform('win32', 'x64').rustTarget, 'x86_64-pc-windows-msvc');
  assert.equal(artifactName(resolvePlatform('linux', 'x64')), `${BINARY_NAME}-x86_64-unknown-linux-gnu.tar.xz`);
  assert.equal(artifactName(resolvePlatform('win32', 'x64')), `${BINARY_NAME}-x86_64-pc-windows-msvc.zip`);
  assert.equal(executableName('win32'), `${BINARY_NAME}.exe`);
  assert.equal(releaseBaseUrl('1.2.3'), 'https://github.com/willh/burn-in-zh-subtitles/releases/download/v1.2.3');
});

test('rejects unsupported platforms with a clear error', () => {
  assert.throws(
    () => resolvePlatform('linux', 'arm64'),
    new RegExp(`supports ${supportedPlatformKeys().join(', ')}`),
  );
});

test('parses and verifies sha256 checksums', () => {
  const dir = sandbox('checksum');
  const file = join(dir, 'sample.txt');
  writeFileSync(file, 'hello world');

  const digest = sha256(file);
  assert.equal(parseChecksum(`${digest}  sample.txt`), digest);
  verifyChecksum(file, `${digest}  sample.txt`);
  assert.throws(() => verifyChecksum(file, `${'0'.repeat(64)}  sample.txt`), /Checksum mismatch/);
  assert.throws(() => parseChecksum('not-a-checksum'), /Invalid checksum file format/);
});

test('finds extracted binaries in nested archive folders', () => {
  const dir = sandbox('archive');
  const nested = join(dir, 'release-root');
  mkdirSync(nested, { recursive: true });
  const binary = join(nested, BINARY_NAME);
  writeFileSync(binary, '');

  assert.equal(findExtractedBinary(dir), binary);
});

test('cli wrapper launches the installed binary on non-Windows hosts', () => {
  if (process.platform === 'win32') {
    return;
  }

  const binDir = installedBinDir();
  const binPath = installedBinPath();
  rmSync(binDir, { recursive: true, force: true });
  mkdirSync(binDir, { recursive: true });
  writeFileSync(
    binPath,
    '#!/usr/bin/env node\nprocess.stdout.write(process.argv.slice(2).join(\" \"));\n',
  );
  chmodSync(binPath, 0o755);

  try {
    const result = spawnSync(process.execPath, ['npm/cli.cjs', 'hello', 'world'], {
      cwd: join(__dirname, '..'),
      encoding: 'utf8',
    });
    assert.equal(result.status, 0);
    assert.equal(result.stdout, 'hello world');
    assert.equal(result.stderr, '');
  } finally {
    rmSync(binDir, { recursive: true, force: true });
  }
});

test('builds the expected release asset URL list', () => {
  const urls = expectedReleaseUrls('9.9.9');
  assert.equal(urls.length, 8);
  assert.ok(urls.includes('https://github.com/willh/burn-in-zh-subtitles/releases/download/v9.9.9/burn-in-zh-subtitles-x86_64-unknown-linux-gnu.tar.xz'));
  assert.ok(urls.includes('https://github.com/willh/burn-in-zh-subtitles/releases/download/v9.9.9/burn-in-zh-subtitles-x86_64-pc-windows-msvc.zip.sha256'));
});

test('verifies release assets and reports failures clearly', async () => {
  const okUrls = await verifyReleaseAssets({
    version: '1.0.0',
    check: async (url) => ({ url, ok: true, statusCode: 200 }),
    retries: 1,
  });
  assert.equal(okUrls.length, 8);

  await assert.rejects(
    verifyReleaseAssets({
      version: '1.0.0',
      check: async (url) => ({ url, ok: url.endsWith('.sha256'), statusCode: url.endsWith('.sha256') ? 200 : 404 }),
      retries: 1,
    }),
    /Missing or unavailable release assets for v1.0.0:/,
  );

  assert.equal(
    formatFailure({ url: 'https://example.test/a', statusCode: 404 }),
    '- https://example.test/a (HTTP 404)',
  );
});
