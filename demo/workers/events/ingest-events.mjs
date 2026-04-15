#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

function loadDefaults() {
  const defaults = {};
  for (const relative of ['.env.generated', '.env.example', 'apps/api/.env', 'apps/api/.env.example']) {
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
  return result.status ?? 1;
}

function sleep(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

function parseList(value) {
  return (value || '')
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function parsePositiveInteger(value, fallback) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    return fallback;
  }
  return parsed;
}

function consumeValue(rawArgs, index, flag) {
  const current = rawArgs[index];
  if (current.startsWith(`${flag}=`)) {
    return { value: current.slice(flag.length + 1), nextIndex: index };
  }
  const next = rawArgs[index + 1];
  if (!next || next.startsWith('-')) {
    console.error(`Missing value for ${flag}.`);
    process.exit(1);
  }
  return { value: next, nextIndex: index + 1 };
}

function printHelp() {
  console.log('Usage: node workers/events/ingest-events.mjs [resource ...] [options]');
  console.log('');
  console.log('Resources can be positional (`contract:rewards`, `token:points`, `account:alice`)');
  console.log('or provided through STELLAR_EVENTS_RESOURCES in your env files.');
  console.log('');
  console.log('Options:');
  console.log('  --once                  Run a single cycle and exit');
  console.log('  --status                Run `stellar forge events status` after each cycle');
  console.log('  --export <path>         Export the persisted store after each cycle');
  console.log('  --network <env>         Override STELLAR_NETWORK');
  console.log('  --count <n>             Override STELLAR_EVENTS_BATCH_SIZE');
  console.log('  --interval-ms <n>       Override STELLAR_EVENTS_POLL_INTERVAL_MS');
  console.log('  --topic <filter>        Forward a topic filter to `events backfill`');
  console.log('  --cursor <cursor>       Bootstrap a single resource from a known cursor');
  console.log('  --start-ledger <n>      Bootstrap backfill from a ledger on the first cycle');
  console.log('  -h, --help              Show this help text');
}

function parseArgs(defaults, rawArgs) {
  const options = {
    once: false,
    status: false,
    exportPath: '',
    network: defaults.STELLAR_NETWORK || 'testnet',
    count: String(parsePositiveInteger(defaults.STELLAR_EVENTS_BATCH_SIZE || '200', 200)),
    intervalMs: parsePositiveInteger(defaults.STELLAR_EVENTS_POLL_INTERVAL_MS || '5000', 5000),
    topics: parseList(defaults.STELLAR_EVENTS_TOPICS),
    startLedger: parsePositiveInteger(defaults.STELLAR_EVENTS_START_LEDGER || '', 0) || null,
    cursor: '',
    resources: [],
  };

  for (let index = 0; index < rawArgs.length; index += 1) {
    const arg = rawArgs[index];
    if (arg === '--once') {
      options.once = true;
      continue;
    }
    if (arg === '--status') {
      options.status = true;
      continue;
    }
    if (arg === '--help' || arg === '-h') {
      printHelp();
      process.exit(0);
    }
    if (arg === '--export' || arg.startsWith('--export=')) {
      const consumed = consumeValue(rawArgs, index, '--export');
      options.exportPath = consumed.value;
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--network' || arg.startsWith('--network=')) {
      const consumed = consumeValue(rawArgs, index, '--network');
      options.network = consumed.value;
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--count' || arg.startsWith('--count=')) {
      const consumed = consumeValue(rawArgs, index, '--count');
      options.count = String(parsePositiveInteger(consumed.value, 200));
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--interval-ms' || arg.startsWith('--interval-ms=')) {
      const consumed = consumeValue(rawArgs, index, '--interval-ms');
      options.intervalMs = parsePositiveInteger(consumed.value, 5000);
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--topic' || arg.startsWith('--topic=')) {
      const consumed = consumeValue(rawArgs, index, '--topic');
      options.topics.push(consumed.value);
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--topics' || arg.startsWith('--topics=')) {
      const consumed = consumeValue(rawArgs, index, '--topics');
      options.topics.push(...parseList(consumed.value));
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--cursor' || arg.startsWith('--cursor=')) {
      const consumed = consumeValue(rawArgs, index, '--cursor');
      options.cursor = consumed.value;
      index = consumed.nextIndex;
      continue;
    }
    if (arg === '--start-ledger' || arg.startsWith('--start-ledger=')) {
      const consumed = consumeValue(rawArgs, index, '--start-ledger');
      options.startLedger = parsePositiveInteger(consumed.value, 0) || null;
      index = consumed.nextIndex;
      continue;
    }
    if (arg.startsWith('-')) {
      console.error(`Unknown option: ${arg}`);
      console.error('Run with --help to see the supported worker flags.');
      process.exit(1);
    }
    options.resources.push(...parseList(arg));
  }

  if (options.resources.length === 0) {
    options.resources.push(...parseList(defaults.STELLAR_EVENTS_RESOURCES));
  }
  options.resources = [...new Set(options.resources)];
  if (options.resources.length > 1 && options.cursor) {
    console.error('`--cursor` only supports a single resource at a time.');
    process.exit(1);
  }
  return options;
}

const defaults = loadDefaults();
const options = parseArgs(defaults, process.argv.slice(2));

if (options.resources.length === 0) {
  console.error('Provide at least one resource like `contract:rewards`, `token:points`, or `account:alice`.');
  console.error('You can also set STELLAR_EVENTS_RESOURCES in apps/api/.env.example.');
  process.exit(1);
}

let bootstrapCursor = options.cursor;
let bootstrapStartLedger = options.startLedger;

async function runCycle(cycle) {
  const startedAt = new Date().toISOString();
  console.log(
    `[events-worker] cycle ${cycle} started at ${startedAt} for ${options.resources.join(', ')} on ${options.network}`,
  );
  for (const resource of options.resources) {
    const args = ['--network', options.network, 'events', 'backfill', resource, '--count', options.count];
    for (const topic of options.topics) {
      args.push('--topic', topic);
    }
    if (bootstrapCursor) {
      args.push('--cursor', bootstrapCursor);
    } else if (bootstrapStartLedger) {
      args.push('--start-ledger', String(bootstrapStartLedger));
    }
    console.log(`[events-worker] ingesting ${resource}`);
    const status = runForge(args);
    if (status !== 0) {
      process.exit(status);
    }
  }

  bootstrapCursor = '';
  bootstrapStartLedger = null;

  if (options.status) {
    const statusArgs = ['--network', options.network, 'events', 'status'];
    console.log('[events-worker] refreshing event store status');
    const status = runForge(statusArgs);
    if (status !== 0) {
      process.exit(status);
    }
  }

  if (options.exportPath) {
    const exportArgs = ['--network', options.network, 'events', 'export', '--path', options.exportPath];
    console.log(`[events-worker] exporting store snapshot to ${options.exportPath}`);
    const status = runForge(exportArgs);
    if (status !== 0) {
      process.exit(status);
    }
  }
}

async function ingestLoop() {
  let cycle = 1;
  do {
    await runCycle(cycle);
    if (options.once) {
      return;
    }
    console.log(`[events-worker] sleeping for ${options.intervalMs}ms`);
    await sleep(options.intervalMs);
    cycle += 1;
  } while (true);
}

await ingestLoop();
