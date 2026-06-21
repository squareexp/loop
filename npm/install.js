#!/usr/bin/env node
/**
 * install.js — runs after npm install.
 *
 * Resolution order:
 *   1. Platform-specific optional package (@loopeng/loop-<platform>)
 *   2. GitHub Releases download (if the optional package is missing)
 *   3. cargo install loop_cli (if Rust is available)
 *   4. Clear error telling the user what to do
 */

'use strict';

const { execSync, spawnSync } = require('child_process');
const https  = require('https');
const fs     = require('fs');
const path   = require('path');
const os     = require('os');
const zlib   = require('zlib');

const VERSION = '0.2.0';
const REPO    = 'squareexp/loop';
const BIN_DIR = path.join(__dirname, 'bin');

const PLATFORM_MAP = {
  'darwin-arm64': 'loop-darwin-arm64',
  'darwin-x64':   'loop-darwin-x64',
  'linux-x64':    'loop-linux-x64',
  'win32-x64':    'loop-win32-x64.exe',
};

function platformKey() {
  return `${os.platform()}-${os.arch()}`;
}

function binName() {
  return os.platform() === 'win32' ? 'loop.exe' : 'loop';
}

function binPath() {
  return path.join(BIN_DIR, binName());
}

// ── 1. Try the optional platform package ──────────────────────────────────────

function tryOptionalPackage() {
  const key = platformKey();
  const pkgName = `@loopeng/loop-${key}`;
  try {
    const pkgDir  = require.resolve(`${pkgName}/package.json`);
    const pkgBin  = path.join(path.dirname(pkgDir), binName());
    if (fs.existsSync(pkgBin)) {
      fs.mkdirSync(BIN_DIR, { recursive: true });
      fs.copyFileSync(pkgBin, binPath());
      fs.chmodSync(binPath(), 0o755);
      console.log(`[loop] Installed from ${pkgName}`);
      return true;
    }
  } catch (_) {}
  return false;
}

// ── 2. Download from GitHub Releases ─────────────────────────────────────────

function githubDownloadUrl() {
  const key = platformKey();
  const names = {
    'darwin-arm64': `loop-${VERSION}-aarch64-apple-darwin.tar.gz`,
    'darwin-x64':   `loop-${VERSION}-x86_64-apple-darwin.tar.gz`,
    'linux-x64':    `loop-${VERSION}-x86_64-unknown-linux-musl.tar.gz`,
    'win32-x64':    `loop-${VERSION}-x86_64-pc-windows-msvc.zip`,
  };
  const name = names[key];
  if (!name) return null;
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${name}`;
}

function downloadBinary(url) {
  return new Promise((resolve, reject) => {
    const tmp = path.join(os.tmpdir(), `loop-download-${Date.now()}`);
    const file = fs.createWriteStream(tmp);

    const get = (url) => {
      https.get(url, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          get(res.headers.location);
          return;
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode}`));
          return;
        }
        res.pipe(file);
        file.on('finish', () => file.close(() => resolve(tmp)));
      }).on('error', reject);
    };

    get(url);
  });
}

async function tryGithubRelease() {
  const url = githubDownloadUrl();
  if (!url) return false;

  console.log(`[loop] Downloading from GitHub Releases...`);
  try {
    const tmp = await downloadBinary(url);
    fs.mkdirSync(BIN_DIR, { recursive: true });

    // For .tar.gz extract the binary
    if (url.endsWith('.tar.gz')) {
      execSync(`tar -xzf "${tmp}" -C "${BIN_DIR}" --strip-components=1 loop`, { stdio: 'inherit' });
    } else {
      // .zip — use unzip
      execSync(`unzip -o "${tmp}" loop.exe -d "${BIN_DIR}"`, { stdio: 'inherit' });
    }

    fs.unlinkSync(tmp);
    fs.chmodSync(binPath(), 0o755);
    console.log(`[loop] Downloaded binary to ${binPath()}`);
    return true;
  } catch (e) {
    console.warn(`[loop] GitHub release download failed: ${e.message}`);
    return false;
  }
}

// ── 3. cargo install fallback ─────────────────────────────────────────────────

function tryCargoInstall() {
  const cargo = spawnSync('cargo', ['--version'], { encoding: 'utf8' });
  if (cargo.status !== 0) return false;

  console.log('[loop] Rust found — building from source with cargo...');
  const result = spawnSync(
    'cargo', ['install', 'loop_cli', '--version', VERSION],
    { stdio: 'inherit', shell: true }
  );
  if (result.status === 0) {
    console.log('[loop] Installed via cargo install');
    return true;
  }
  return false;
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  if (fs.existsSync(binPath())) {
    console.log(`[loop] Binary already present at ${binPath()}`);
    return;
  }

  if (tryOptionalPackage()) return;
  if (await tryGithubRelease()) return;
  if (tryCargoInstall()) return;

  console.error(`
[loop] Could not install the loop binary automatically.

Options:
  1. Install Rust (https://rustup.rs) and run: cargo install loop_cli
  2. Download the binary for your platform from:
     https://github.com/${REPO}/releases/tag/v${VERSION}
     and put it somewhere on your PATH named "loop"
`);
  process.exit(1);
}

main().catch((e) => {
  console.error('[loop] Install failed:', e.message);
  process.exit(1);
});
