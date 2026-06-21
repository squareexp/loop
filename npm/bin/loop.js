#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const path = require('path');
const fs   = require('fs');
const os   = require('os');

const binName = os.platform() === 'win32' ? 'loop.exe' : 'loop';
const localBin = path.join(__dirname, binName);

// Try the locally downloaded binary first
if (fs.existsSync(localBin)) {
  const result = spawnSync(localBin, process.argv.slice(2), { stdio: 'inherit' });
  process.exit(result.status ?? 1);
}

// Fall back to PATH (e.g., installed via cargo install)
const result = spawnSync('loop', process.argv.slice(2), {
  stdio: 'inherit',
  shell: true,
});

if (result.error) {
  console.error(
    '[loop] Binary not found. Run: npm install -g @loopeng/loop\n' +
    'Or install Rust and run: cargo install loop_cli'
  );
  process.exit(1);
}

process.exit(result.status ?? 1);
