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
const rawArgs = process.argv.slice(2);
let mode = 'deploy';
let targetEnv = defaults.STELLAR_NETWORK || 'testnet';
let envExplicit = false;
const forwarded = [];

for (const arg of rawArgs) {
  if (arg === '--plan') {
    mode = 'plan';
    continue;
  }
  if (arg === '--verify') {
    mode = 'verify';
    continue;
  }
  if (arg === '--env-export') {
    mode = 'env-export';
    continue;
  }
  if (arg === '--aliases-sync') {
    mode = 'aliases-sync';
    continue;
  }
  if (!arg.startsWith('-') && !envExplicit) {
    targetEnv = arg;
    envExplicit = true;
    continue;
  }
  forwarded.push(arg);
}

const args =
  mode === 'plan'
    ? ['release', 'plan', targetEnv, ...forwarded]
    : mode === 'verify'
      ? ['release', 'verify', targetEnv, ...forwarded]
      : mode === 'env-export'
        ? ['release', 'env', 'export', targetEnv, ...forwarded]
        : mode === 'aliases-sync'
          ? ['release', 'aliases', 'sync', targetEnv, ...forwarded]
          : ['release', 'deploy', targetEnv, ...forwarded];

runForge(args);
