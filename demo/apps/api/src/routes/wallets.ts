import type { FastifyInstance } from 'fastify';
import { manifest } from '../lib/manifest.js';

export function registerWalletRoutes(app: FastifyInstance) {
  app.get('/wallets', async () => ({
    wallets: Object.entries(manifest.wallets).map(([name, wallet]) => ({
      name,
      kind: wallet.kind,
      identity: wallet.kind === 'classic' ? wallet.identity : null,
      controller_identity:
        wallet.controller_identity ??
        (wallet.kind === 'smart' && wallet.identity ? wallet.identity : null),
      mode: wallet.mode ?? null,
      onboarding_app: wallet.onboarding_app ?? null,
      policy_contract: wallet.policy_contract ?? null,
    })),
  }));

  app.get('/wallets/:name', async (request) => {
    const params = request.params as { name: string };
    return {
      name: params.name,
      wallet: manifest.wallets[params.name as keyof typeof manifest.wallets] ?? null,
    };
  });
}
