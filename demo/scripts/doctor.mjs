#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function loadDefaults() {
  const defaults = {};
  for (const relative of ['.env.generated', '.env.example']) {
    const file = resolve(root, relative);
    if (!existsSync(file)) {
      continue;
    }
    for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      const separator = trimmed.indexOf('=');
      if (separator === -1) {
        continue;
      }
      const key = trimmed.slice(0, separator).trim();
      const value = trimmed.slice(separator + 1).trim();
      if (value && !(key in defaults)) {
        defaults[key] = value;
      }
    }
  }
  return defaults;
}

function runForge(args) {
  const command = process.env.STELLAR_FORGE_BIN || 'stellar-forge';
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
  if (result.error) {
    if (result.error.code === 'ENOENT') {
      console.error(`Could not find ${command} on PATH.`);
      console.error('Set STELLAR_FORGE_BIN to override the executable name.');
      process.exit(1);
    }
    throw result.error;
  }
  process.exit(result.status ?? 1);
}

const defaults = loadDefaults();
const network = defaults.STELLAR_NETWORK || 'testnet';
const userArgs = process.argv.slice(2);
const hasNetworkFlag = userArgs.some((arg) => arg === '--network' || arg.startsWith('--network='));
const args = hasNetworkFlag
  ? ['doctor', ...userArgs]
  : ['--network', network, 'doctor', ...userArgs];

runForge(args);
