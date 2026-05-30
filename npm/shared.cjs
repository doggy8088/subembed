'use strict';

const { join } = require('node:path');

const PACKAGE_ROOT = join(__dirname, '..');
const PACKAGE_NAME = 'subembed';
const CLI_NAME = 'subembed';
const BINARY_NAME = 'subembed';
const LOCAL_BUILD_BINARY_NAME = 'subembed';
const GITHUB_OWNER = 'doggy8088';
const GITHUB_REPO = 'subembed';

const SUPPORTED_TARGETS = {
  'darwin-arm64': {
    platform: 'darwin',
    arch: 'arm64',
    rustTarget: 'aarch64-apple-darwin',
    archiveExt: 'tar.xz',
  },
  'darwin-x64': {
    platform: 'darwin',
    arch: 'x64',
    rustTarget: 'x86_64-apple-darwin',
    archiveExt: 'tar.xz',
  },
  'linux-x64': {
    platform: 'linux',
    arch: 'x64',
    rustTarget: 'x86_64-unknown-linux-gnu',
    archiveExt: 'tar.xz',
  },
  'win32-x64': {
    platform: 'win32',
    arch: 'x64',
    rustTarget: 'x86_64-pc-windows-msvc',
    archiveExt: 'zip',
  },
};

function platformKey(platform = process.platform, arch = process.arch) {
  return `${platform}-${arch}`;
}

function supportedPlatformKeys() {
  return Object.keys(SUPPORTED_TARGETS).sort();
}

function resolvePlatform(platform = process.platform, arch = process.arch) {
  const spec = SUPPORTED_TARGETS[platformKey(platform, arch)];
  if (!spec) {
    throw new Error(
      `Unsupported platform ${platform}/${arch}. ${PACKAGE_NAME} supports ${supportedPlatformKeys().join(', ')}.`,
    );
  }
  return spec;
}

function artifactName(specOrTarget) {
  if (typeof specOrTarget === 'string') {
    const archiveExt = specOrTarget.includes('windows') ? 'zip' : 'tar.xz';
    return `${BINARY_NAME}-${specOrTarget}.${archiveExt}`;
  }
  return `${BINARY_NAME}-${specOrTarget.rustTarget}.${specOrTarget.archiveExt}`;
}

function executableName(platform = process.platform) {
  return platform === 'win32' ? `${BINARY_NAME}.exe` : BINARY_NAME;
}

function localBuildExecutableName(platform = process.platform) {
  return platform === 'win32' ? `${LOCAL_BUILD_BINARY_NAME}.exe` : LOCAL_BUILD_BINARY_NAME;
}

function installedBinDir() {
  return join(__dirname, `${CLI_NAME}-bin`);
}

function installedBinPath(platform = process.platform) {
  return join(installedBinDir(), executableName(platform));
}

function releaseTag(version) {
  return `v${version}`;
}

function releaseBaseUrl(version) {
  return `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/download/${releaseTag(version)}`;
}

module.exports = {
  BINARY_NAME,
  CLI_NAME,
  GITHUB_OWNER,
  GITHUB_REPO,
  LOCAL_BUILD_BINARY_NAME,
  PACKAGE_NAME,
  PACKAGE_ROOT,
  SUPPORTED_TARGETS,
  artifactName,
  executableName,
  installedBinDir,
  installedBinPath,
  localBuildExecutableName,
  platformKey,
  releaseBaseUrl,
  releaseTag,
  resolvePlatform,
  supportedPlatformKeys,
};
