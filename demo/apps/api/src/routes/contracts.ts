import type { FastifyInstance } from 'fastify';
import * as contractService0 from '../services/contracts/app.js';

export function registerContractRoutes(app: FastifyInstance) {
  app.post('/contracts/app/call/:fn', async (request) => {
    const params = request.params as { fn: string };
    return contractService0.preview(params.fn, request.body);
  });

  app.post('/contracts/app/tx/:fn', async (request) => {
    const params = request.params as { fn: string };
    return contractService0.buildTx(params.fn, request.body);
  });

}