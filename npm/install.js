#!/usr/bin/env node
/**
 * install.js — downloads or builds the loop binary after npm install.
 *
 * Resolution order:
 *   1. Already present (skip)
 *   2. Platform-specific optional package (@loopeng/loop-<platform>)
 *   3. cargo install loop_cli   (if Rust is installed)
 *   4. Clear instructions if neither is available
 */

'use strict';

const { spawnSync } = require('child_process');
const fs   = require('fs');
const path = require('path');
const os   = require('os');

const BIN_DIR  = path.join(__dirname, 'bin');
const BIN_NAME = os.platform() === 'win32' ? 'loop.exe' : 'loop';
const BIN_PATH = path.join(BIN_DIR, BIN_NAME);

// ── Already installed? ────────────────────────────────────────────────────────

if (fs.existsSync(BIN_PATH)) {
  console.log('[loop] binary already present, skipping install.');
  process.exit(0);
}

// ── 1. Optional platform package ─────────────────────────────────────────────

function tryOptionalPackage() {
  const key     = `${os.platform()}-${os.arch()}`;
  const pkgName = `@loopeng/loop-${key}`;
  try {
    const pkgJson = require.resolve(`${pkgName}/package.json`);
    const binSrc  = path.join(path.dirname(pkgJson), BIN_NAME);
    if (fs.existsSync(binSrc)) {
      fs.mkdirSync(BIN_DIR, { recursive: true });
      fs.copyFileSync(binSrc, BIN_PATH);
      if (os.platform() !== 'win32') fs.chmodSync(BIN_PATH, 0o755);
      console.log(`[loop] installed from ${pkgName}`);
      return true;
    }
  } catch (_) {}
  return false;
}

// ── 2. cargo install ─────────────────────────────────────────────────────────

function tryCargoInstall() {
  const check = spawnSync('cargo', ['--version'], { encoding: 'utf8' });
  if (check.status !== 0) return false;

  console.log('[loop] building from source with cargo (this takes ~1 minute)...');

  const result = spawnSync(
    'cargo', ['install', 'loop_cli', '--version', '0.2.0'],
    { stdio: 'inherit', shell: true }
  );

  if (result.status === 0) {
    // cargo puts the binary on PATH, not in our bin/ — find it and copy
    const cargoHome = process.env.CARGO_HOME || path.join(os.homedir(), '.cargo');
    const cargoLoop = path.join(cargoHome, 'bin', BIN_NAME);
    if (fs.existsSync(cargoLoop)) {
      fs.mkdirSync(BIN_DIR, { recursive: true });
      fs.copyFileSync(cargoLoop, BIN_PATH);
      if (os.platform() !== 'win32') fs.chmodSync(BIN_PATH, 0o755);
    }
    console.log('[loop] installed via cargo install loop_cli');
    return true;
  }

  return false;
}

// ── Main ─────────────────────────────────────────────────────────────────────

if (tryOptionalPackage()) process.exit(0);
if (tryCargoInstall())    process.exit(0);

console.error(`
[loop] Could not install the loop binary automatically.

To fix this, install Rust and then run:

    cargo install loop_cli

Or download a prebuilt binary from:

    https://github.com/squareexp/loop/releases

and put it somewhere on your PATH named "loop".
`);
process.exit(1);
