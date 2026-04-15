import type { FastifyInstance } from 'fastify';
import { HttpError } from '../lib/errors.js';
import { relayerStatus, submitSponsoredTransaction } from '../services/relayer.js';

export function registerRelayerRoutes(app: FastifyInstance) {
  app.get('/relayer/status', async () => relayerStatus());

  app.post('/relayer/submit', async (request, reply) => {
    try {
      const payload =
        request.body && typeof request.body === 'object' && !Array.isArray(request.body)
          ? (request.body as Record<string, unknown>)
          : {};
      return await submitSponsoredTransaction(payload);
    } catch (error) {
      if (error instanceof HttpError) {
        reply.code(error.statusCode);
        return {
          accepted: false,
          error: error.message,
          detail: error.detail,
        };
      }
      throw error;
    }
  });
}
