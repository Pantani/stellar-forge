import type { FastifyInstance } from 'fastify';
import { loadApiConfig } from '../lib/config.js';
import { manifest } from '../lib/manifest.js';

export function registerHealthRoutes(app: FastifyInstance) {
  app.get('/health', async () => ({
    status: 'ok',
    project: manifest.project.slug,
    network: loadApiConfig().network,
  }));

  app.get('/ready', async () => ({
    ready: true,
    contracts: Object.keys(manifest.contracts).length,
    tokens: Object.keys(manifest.tokens).length,
  }));

  app.get('/version', async () => ({
    version: manifest.project.version,
  }));
}
