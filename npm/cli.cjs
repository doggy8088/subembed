#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');

const { BINARY_NAME, CLI_NAME, PACKAGE_NAME, installedBinPath } = require('./shared.cjs');

const bin = installedBinPath();

if (!existsSync(bin)) {
  console.error(
    `${PACKAGE_NAME} could not find the installed ${BINARY_NAME} binary. Reinstall ${PACKAGE_NAME} to restore the native executable for ${CLI_NAME}.`,
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
