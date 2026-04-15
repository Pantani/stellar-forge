import type { FastifyInstance } from 'fastify';
import { getEventStatus, listEventCursors, resolveEventPaths, resolveEventWorkerConfig } from '../lib/events-store.js';
import { manifest } from '../lib/manifest.js';

function trackedResources(filters: string[]) {
  const declared = [
    ...Object.keys(manifest.contracts).map((name) => `contract:${name}`),
    ...Object.keys(manifest.tokens).map((name) => `token:${name}`),
  ];
  if (filters.length === 0) {
    return declared;
  }
  return declared.filter((resource) => {
    const [, name] = resource.split(':');
    return filters.includes(resource) || (name ? filters.includes(name) : false);
  });
}

export function registerEventRoutes(app: FastifyInstance) {
  app.get('/events/status', async () => {
    const status = getEventStatus();
    const worker = resolveEventWorkerConfig();
    const activeBackend = manifest.api?.events_backend ?? 'rpc-poller';
    const retentionDays = worker.retention_days ?? (activeBackend === 'rpc-poller' ? 7 : null);
    return {
      backend: activeBackend,
      database: manifest.api?.database ?? 'sqlite',
      db_path: resolveEventPaths().dbPath,
      contracts: Object.keys(manifest.contracts),
      tokens: Object.keys(manifest.tokens),
      tracked_resources: trackedResources(worker.resources),
      worker,
      retention_days: retentionDays,
      retention_warning: retentionDays === null
        ? null
        : `RPC/event retention is short; backfill older than ${retentionDays} day(s) requires your own archive or indexer.`,
      ...status,
      cursor_names: listEventCursors().map((cursor) => cursor.name),
    };
  });

  app.get('/events/cursors', async () => ({
    db_path: resolveEventPaths().dbPath,
    cursors: listEventCursors(),
  }));
}
