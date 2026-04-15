import { execFile } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { config as loadDotenv } from 'dotenv';
import { manifest } from '../lib/manifest.js';
import {
  insertEvent,
  loadCursor,
  openEventStore,
  resolveEventWorkerConfig,
  syncCursorSnapshot,
  upsertCursor,
} from '../lib/events-store.js';

type TrackedResource = {
  kind: 'contract' | 'token';
  name: string;
  contractId: string;
};

type RawEvent = Record<string, unknown>;

const execFileAsync = promisify(execFile);
const workerFile = fileURLToPath(import.meta.url);
const apiRoot = path.resolve(path.dirname(workerFile), '../..');
const projectRoot = path.resolve(apiRoot, '..', '..');

function loadForgeEnv() {
  const candidates = [
    path.join(apiRoot, '.env'),
    path.join(apiRoot, '.env.local'),
    path.join(projectRoot, '.env'),
    path.join(projectRoot, '.env.local'),
    path.join(projectRoot, '.env.generated'),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      loadDotenv({ path: candidate, override: false });
    }
  }
}

function shouty(value: string) {
  return value.replace(/[^A-Za-z0-9]+/g, '_').replace(/^_+|_+$/g, '').toUpperCase();
}

function nonEmpty(values: Array<string | undefined>) {
  return values.find((value) => typeof value === 'string' && value.length > 0);
}

function deployedResourceId(kind: 'contract' | 'token', name: string) {
  const key = shouty(name);
  if (kind === 'contract') {
    return nonEmpty([
      process.env[`PUBLIC_${key}_CONTRACT_ID`],
      process.env[`STELLAR_${key}_CONTRACT_ID`],
    ]);
  }
  return nonEmpty([
    process.env[`PUBLIC_${key}_SAC_ID`],
    process.env[`STELLAR_${key}_SAC_ID`],
    process.env[`PUBLIC_${key}_TOKEN_ID`],
    process.env[`STELLAR_${key}_TOKEN_ID`],
  ]);
}

function trackedResources(resourceFilters: string[]): TrackedResource[] {
  const allow = new Set(resourceFilters);
  const matches = (kind: 'contract' | 'token', name: string) =>
    allow.size === 0 || allow.has(name) || allow.has(`${kind}:${name}`);

  const contracts = Object.keys(manifest.contracts)
    .filter((name) => matches('contract', name))
    .map((name) => ({
      kind: 'contract' as const,
      name,
      contractId: deployedResourceId('contract', name) ?? '',
    }))
    .filter((resource) => resource.contractId.length > 0);
  const tokens = Object.keys(manifest.tokens)
    .filter((name) => matches('token', name))
    .map((name) => ({
      kind: 'token' as const,
      name,
      contractId: deployedResourceId('token', name) ?? '',
    }))
    .filter((resource) => resource.contractId.length > 0);
  return [...contracts, ...tokens];
}

function activeNetwork() {
  return process.env.STELLAR_NETWORK ?? manifest.defaults.network;
}

function looksLikeXdrSegment(value: string) {
  if (value.length < 8 || value.length % 4 !== 0) {
    return false;
  }
  try {
    const decoded = Buffer.from(value, 'base64');
    return decoded.length >= 8 && decoded.length % 4 === 0;
  } catch {
    return false;
  }
}

function looksLikeSymbol(value: string) {
  return /^[A-Za-z0-9_]{1,32}$/.test(value);
}

function encodeScValWithPayload(tag: number, payload: Buffer) {
  const header = Buffer.alloc(4);
  header.writeUInt32BE(tag, 0);
  return Buffer.concat([header, payload]).toString('base64');
}

function encodeScValStringLike(tag: number, value: string) {
  const bytes = Buffer.from(value, 'utf8');
  const size = Buffer.alloc(4);
  size.writeUInt32BE(bytes.length, 0);
  const padding = Buffer.alloc((4 - (bytes.length % 4)) % 4);
  return encodeScValWithPayload(tag, Buffer.concat([size, bytes, padding]));
}

function encodeTopicSegment(segment: string) {
  if (segment === '*' || segment === '**' || looksLikeXdrSegment(segment)) {
    return segment;
  }

  if (segment.startsWith('sym:') || segment.startsWith('symbol:')) {
    const value = segment.startsWith('sym:')
      ? segment.slice('sym:'.length)
      : segment.slice('symbol:'.length);
    if (!looksLikeSymbol(value)) {
      throw new Error(`invalid symbol topic segment \`${value}\``);
    }
    return encodeScValStringLike(15, value);
  }
  if (segment.startsWith('str:') || segment.startsWith('string:')) {
    const value = segment.startsWith('str:')
      ? segment.slice('str:'.length)
      : segment.slice('string:'.length);
    return encodeScValStringLike(14, value);
  }
  if (segment.startsWith('bool:')) {
    const value = segment.slice('bool:'.length);
    if (value !== 'true' && value !== 'false') {
      throw new Error(`invalid bool topic segment \`${value}\``);
    }
    return encodeScValWithPayload(0, Buffer.from([0, 0, 0, value === 'true' ? 1 : 0]));
  }
  if (segment.startsWith('u32:')) {
    const value = Number.parseInt(segment.slice('u32:'.length), 10);
    if (!Number.isFinite(value) || value < 0) {
      throw new Error(`invalid u32 topic segment \`${segment}\``);
    }
    const payload = Buffer.alloc(4);
    payload.writeUInt32BE(value, 0);
    return encodeScValWithPayload(3, payload);
  }
  if (segment.startsWith('i32:')) {
    const value = Number.parseInt(segment.slice('i32:'.length), 10);
    if (!Number.isFinite(value)) {
      throw new Error(`invalid i32 topic segment \`${segment}\``);
    }
    const payload = Buffer.alloc(4);
    payload.writeInt32BE(value, 0);
    return encodeScValWithPayload(4, payload);
  }
  if (segment.startsWith('u64:')) {
    const payload = Buffer.alloc(8);
    payload.writeBigUInt64BE(BigInt(segment.slice('u64:'.length)), 0);
    return encodeScValWithPayload(5, payload);
  }
  if (segment.startsWith('i64:')) {
    const payload = Buffer.alloc(8);
    payload.writeBigInt64BE(BigInt(segment.slice('i64:'.length)), 0);
    return encodeScValWithPayload(6, payload);
  }

  if (looksLikeSymbol(segment)) {
    return encodeScValStringLike(15, segment);
  }
  return encodeScValStringLike(14, segment);
}

function normalizeTopicFilter(filter: string) {
  const segments = filter
    .split(',')
    .map((segment) => segment.trim())
    .filter((segment) => segment.length > 0);
  if (segments.length === 0) {
    throw new Error(`topic filter \`${filter}\` is empty`);
  }
  if (segments.length > 5) {
    throw new Error(
      `topic filter \`${filter}\` has too many segments; use up to 4 plus an optional trailing **`,
    );
  }
  const deepWildcard = segments.indexOf('**');
  if (deepWildcard >= 0 && deepWildcard !== segments.length - 1) {
    throw new Error(
      `topic filter \`${filter}\` uses ** before the end; it must be the last segment`,
    );
  }
  return segments.map(encodeTopicSegment).join(',');
}

function extractEvents(payload: unknown): RawEvent[] {
  if (Array.isArray(payload)) {
    return payload.filter((entry): entry is RawEvent => typeof entry === 'object' && entry !== null);
  }
  if (payload && typeof payload === 'object') {
    const record = payload as Record<string, unknown>;
    if (Array.isArray(record.events)) {
      return record.events.filter((entry): entry is RawEvent => typeof entry === 'object' && entry !== null);
    }
    if (Array.isArray(record.records)) {
      return record.records.filter((entry): entry is RawEvent => typeof entry === 'object' && entry !== null);
    }
    if (
      record.result &&
      typeof record.result === 'object' &&
      Array.isArray((record.result as Record<string, unknown>).events)
    ) {
      return ((record.result as Record<string, unknown>).events as unknown[]).filter(
        (entry): entry is RawEvent => typeof entry === 'object' && entry !== null,
      );
    }
    if (
      record.result &&
      typeof record.result === 'object' &&
      Array.isArray((record.result as Record<string, unknown>).records)
    ) {
      return ((record.result as Record<string, unknown>).records as unknown[]).filter(
        (entry): entry is RawEvent => typeof entry === 'object' && entry !== null,
      );
    }
  }
  return [];
}

function eventString(event: RawEvent, keys: string[]) {
  for (const key of keys) {
    const value = event[key];
    if (typeof value === 'string' && value.length > 0) {
      return value;
    }
  }
  return undefined;
}

function eventNumber(event: RawEvent, keys: string[]) {
  for (const key of keys) {
    const value = event[key];
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value;
    }
    if (typeof value === 'string' && value.length > 0) {
      const parsed = Number.parseInt(value, 10);
      if (Number.isFinite(parsed)) {
        return parsed;
      }
    }
  }
  return undefined;
}

function eventTopic(event: RawEvent) {
  return Array.isArray(event.topic)
    ? event.topic
    : Array.isArray(event.topics)
      ? event.topics
      : [];
}

function eventPayload(event: RawEvent) {
  return event.value ?? event.data ?? event.payload ?? event.body ?? null;
}

function normalizeEvent(resource: TrackedResource, cursorName: string, event: RawEvent) {
  const externalId =
    eventString(event, ['id']) ??
    `${resource.kind}:${resource.name}:${eventString(event, ['txHash', 'tx_hash']) ?? 'unknown'}:${eventNumber(event, ['ledger', 'ledgerSequence', 'ledger_sequence']) ?? 0}:${eventString(event, ['cursor', 'pagingToken', 'paging_token', 'pagingTokenId']) ?? 'tail'}`;
  const cursor =
    eventString(event, ['cursor', 'pagingToken', 'paging_token', 'pagingTokenId', 'id']) ?? null;
  const ledger =
    eventNumber(event, ['ledger', 'ledgerSequence', 'ledger_sequence']) ?? null;
  return {
    external_id: externalId,
    cursor_name: cursorName,
    cursor,
    resource_kind: resource.kind,
    resource_name: resource.name,
    contract_id:
      eventString(event, ['contractId', 'contract_id']) ?? resource.contractId,
    event_type: eventString(event, ['type', 'eventType', 'event_type']) ?? 'contract',
    topic: JSON.stringify(eventTopic(event)),
    payload: JSON.stringify(eventPayload(event)),
    tx_hash: eventString(event, ['txHash', 'tx_hash']) ?? null,
    ledger,
    observed_at:
      eventString(event, ['ledgerClosedAt', 'ledger_closed_at']) ??
      new Date().toISOString(),
  };
}

async function fetchEvents(
  resource: TrackedResource,
  cursor: string | null,
  config: ReturnType<typeof resolveEventWorkerConfig>,
  topicFilters: string[],
  startLedger?: number,
) {
  const args = [
    'events',
    '--output',
    'json',
    '--count',
    String(Math.min(config.batch_size, 1000)),
    '--id',
    resource.contractId,
    '--network',
    activeNetwork(),
  ];

  if (config.event_type !== 'all') {
    args.push('--type', config.event_type);
  }
  for (const topic of topicFilters) {
    args.push('--topic', topic);
  }

  if (cursor && cursor.length > 0) {
    args.push('--cursor', cursor);
  } else if (startLedger) {
    args.push('--start-ledger', String(startLedger));
  }

  const { stdout } = await execFileAsync(
    process.env.STELLAR_BIN ?? 'stellar',
    args,
    {
      cwd: projectRoot,
      maxBuffer: 16 * 1024 * 1024,
    },
  );

  return extractEvents(JSON.parse(stdout));
}

export async function ingestOnce() {
  loadForgeEnv();
  const config = resolveEventWorkerConfig();
  const topicFilters = config.topic_filters.map(normalizeTopicFilter);
  const db = openEventStore();
  try {
    const resources = trackedResources(config.resources);
    if (resources.length === 0) {
      console.warn('No deployed contracts or token wrappers found in env; skipping event ingest.');
      syncCursorSnapshot(db);
      return;
    }

    for (const resource of resources) {
      const cursorName = `${activeNetwork()}:${resource.kind}:${resource.name}`;
      const previous = loadCursor(cursorName, db);
      const events = await fetchEvents(
        resource,
        previous?.cursor ?? null,
        config,
        topicFilters,
        previous?.last_ledger ?? config.start_ledger ?? undefined,
      );

      let lastCursor = previous?.cursor ?? null;
      let lastLedger = previous?.last_ledger ?? null;

      for (const rawEvent of events) {
        const normalized = normalizeEvent(resource, cursorName, rawEvent);
        insertEvent(normalized, db);
        lastCursor = normalized.cursor ?? lastCursor;
        lastLedger = normalized.ledger ?? lastLedger;
      }

      if (events.length > 0) {
        upsertCursor(
          {
            name: cursorName,
            resource_kind: resource.kind,
            resource_name: resource.name,
            cursor: lastCursor,
            last_ledger: lastLedger,
            updated_at: new Date().toISOString(),
          },
          db,
        );
      }

      console.log(
        `[events] ${resource.kind}:${resource.name} -> ${events.length} event(s)`,
      );
    }

    syncCursorSnapshot(db);
  } finally {
    db.close();
  }
}

async function main() {
  const once = process.argv.includes('--once');
  if (once) {
    await ingestOnce();
    return;
  }

  const intervalMs = resolveEventWorkerConfig().poll_interval_ms;
  while (true) {
    await ingestOnce();
    await delay(intervalMs);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
