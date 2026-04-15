create table if not exists cursors (
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
