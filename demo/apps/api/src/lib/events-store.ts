import Database from 'better-sqlite3';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export type EventCursorRow = {
  name: string;
  resource_kind: string;
  resource_name: string;
  cursor: string | null;
  last_ledger: number | null;
  updated_at: string;
};

export type NormalizedEvent = {
  external_id: string;
  cursor_name: string;
  cursor: string | null;
  resource_kind: string;
  resource_name: string;
  contract_id: string;
  event_type: string;
  topic: string;
  payload: string;
  tx_hash: string | null;
  ledger: number | null;
  observed_at: string;
};

export type EventWorkerConfig = {
  batch_size: number;
  poll_interval_ms: number;
  start_ledger: number | null;
  resources: string[];
  topic_filters: string[];
  event_type: 'all' | 'contract' | 'system';
  retention_days: number | null;
};

const apiRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const projectRoot = path.resolve(apiRoot, '..', '..');
const fallbackSchema = `create table if not exists cursors (
  name text primary key,
  resource_kind text not null,
  resource_name text not null,
  cursor text,
  last_ledger integer,
  updated_at text not null
);
create table if not exists events (
  id integer primary key autoincrement,
  external_id text not null unique,
  cursor_name text not null,
  cursor text,
  resource_kind text not null,
  resource_name text not null,
  contract_id text not null,
  event_type text not null,
  topic text not null,
  payload text not null,
  tx_hash text,
  ledger integer,
  observed_at text not null
);
create index if not exists idx_events_cursor_name on events (cursor_name, ledger desc);
create index if not exists idx_events_contract_id on events (contract_id, ledger desc);
`;

function resolvePath(base: string, candidate: string) {
  return path.isAbsolute(candidate) ? candidate : path.resolve(base, candidate);
}

export function resolveEventPaths() {
  const dbPath = resolvePath(
    apiRoot,
    process.env.STELLAR_EVENTS_DB_PATH ?? './db/events.sqlite',
  );
  const schemaPath = resolvePath(
    apiRoot,
    process.env.STELLAR_EVENTS_SCHEMA_PATH ?? './db/schema.sql',
  );
  const cursorSnapshotPath = resolvePath(
    apiRoot,
    process.env.STELLAR_EVENTS_CURSOR_FILE ?? '../../workers/events/cursors.json',
  );
  return { dbPath, schemaPath, cursorSnapshotPath };
}

function parsePositiveInteger(value: string | undefined) {
  if (!value || value.length === 0) {
    return null;
  }
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }
  return parsed;
}

function parseList(value: string | undefined) {
  if (!value) {
    return [];
  }
  return value
    .split(/\r?\n|,/)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function parseTopicList(value: string | undefined) {
  if (!value) {
    return [];
  }
  return value
    .split(/\r?\n|;/)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function parseEventType(value: string | undefined): EventWorkerConfig['event_type'] {
  if (value === 'contract' || value === 'system') {
    return value;
  }
  return 'all';
}

export function resolveEventWorkerConfig(): EventWorkerConfig {
  return {
    batch_size: parsePositiveInteger(process.env.STELLAR_EVENTS_BATCH_SIZE) ?? 200,
    poll_interval_ms:
      parsePositiveInteger(process.env.STELLAR_EVENTS_POLL_INTERVAL_MS) ?? 5000,
    start_ledger: parsePositiveInteger(process.env.STELLAR_EVENTS_START_LEDGER),
    resources: parseList(process.env.STELLAR_EVENTS_RESOURCES),
    topic_filters: parseTopicList(process.env.STELLAR_EVENTS_TOPICS),
    event_type: parseEventType(process.env.STELLAR_EVENTS_TYPE),
    retention_days: parsePositiveInteger(process.env.STELLAR_EVENTS_RETENTION_DAYS),
  };
}

function ensureSchema(db: Database.Database) {
  const { schemaPath } = resolveEventPaths();
  const schema = fs.existsSync(schemaPath)
    ? fs.readFileSync(schemaPath, 'utf8')
    : fallbackSchema;
  db.exec(schema);
}

function withStore<T>(store: Database.Database | undefined, fn: (db: Database.Database) => T): T {
  const db = store ?? openEventStore();
  try {
    return fn(db);
  } finally {
    if (!store) {
      db.close();
    }
  }
}

export function openEventStore() {
  const { dbPath } = resolveEventPaths();
  fs.mkdirSync(path.dirname(dbPath), { recursive: true });
  const db = new Database(dbPath);
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  ensureSchema(db);
  return db;
}

export function loadCursor(name: string, store?: Database.Database): EventCursorRow | null {
  return withStore(store, (db) => {
    const row = db
      .prepare(
        `select name, resource_kind, resource_name, cursor, last_ledger, updated_at
         from cursors
         where name = ?`,
      )
      .get(name) as EventCursorRow | undefined;
    return row ?? null;
  });
}

export function upsertCursor(row: EventCursorRow, store?: Database.Database) {
  return withStore(store, (db) => {
    db.prepare(
      `insert into cursors (name, resource_kind, resource_name, cursor, last_ledger, updated_at)
       values (@name, @resource_kind, @resource_name, @cursor, @last_ledger, @updated_at)
       on conflict(name) do update set
         resource_kind = excluded.resource_kind,
         resource_name = excluded.resource_name,
         cursor = excluded.cursor,
         last_ledger = excluded.last_ledger,
         updated_at = excluded.updated_at`,
    ).run(row);
  });
}

export function insertEvent(event: NormalizedEvent, store?: Database.Database) {
  return withStore(store, (db) => {
    db.prepare(
      `insert or ignore into events (
        external_id,
        cursor_name,
        cursor,
        resource_kind,
        resource_name,
        contract_id,
        event_type,
        topic,
        payload,
        tx_hash,
        ledger,
        observed_at
      ) values (
        @external_id,
        @cursor_name,
        @cursor,
        @resource_kind,
        @resource_name,
        @contract_id,
        @event_type,
        @topic,
        @payload,
        @tx_hash,
        @ledger,
        @observed_at
      )`,
    ).run(event);
  });
}

export function listEventCursors(store?: Database.Database): EventCursorRow[] {
  const { dbPath } = resolveEventPaths();
  if (!store && !fs.existsSync(dbPath)) {
    return [];
  }
  return withStore(store, (db) => {
    return db
      .prepare(
        `select name, resource_kind, resource_name, cursor, last_ledger, updated_at
         from cursors
         order by name asc`,
      )
      .all() as EventCursorRow[];
  });
}

export function getEventStatus(store?: Database.Database) {
  const { dbPath } = resolveEventPaths();
  if (!store && !fs.existsSync(dbPath)) {
    return {
      total_events: 0,
      latest_ledger: null,
      latest_observed_at: null,
      cursor_count: 0,
    };
  }
  return withStore(store, (db) => {
    const summary = db
      .prepare(
        `select
           count(*) as total_events,
           max(ledger) as latest_ledger,
           max(observed_at) as latest_observed_at
         from events`,
      )
      .get() as {
      total_events: number;
      latest_ledger: number | null;
      latest_observed_at: string | null;
    };
    const cursorSummary = db
      .prepare(`select count(*) as cursor_count from cursors`)
      .get() as { cursor_count: number };
    return {
      total_events: summary.total_events ?? 0,
      latest_ledger: summary.latest_ledger ?? null,
      latest_observed_at: summary.latest_observed_at ?? null,
      cursor_count: cursorSummary.cursor_count ?? 0,
    };
  });
}

export function syncCursorSnapshot(store?: Database.Database) {
  const rows = listEventCursors(store);
  const { cursorSnapshotPath } = resolveEventPaths();
  fs.mkdirSync(path.dirname(cursorSnapshotPath), { recursive: true });
  const cursors = Object.fromEntries(
    rows.map((row) => [
      row.name,
      {
        cursor: row.cursor,
        last_ledger: row.last_ledger,
        updated_at: row.updated_at,
      },
    ]),
  );
  fs.writeFileSync(
    cursorSnapshotPath,
    JSON.stringify({ cursors }, null, 2) + '\n',
    'utf8',
  );
  return cursorSnapshotPath;
}
