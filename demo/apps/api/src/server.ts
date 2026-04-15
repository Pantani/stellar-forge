import Fastify from 'fastify';
import { registerContractRoutes } from './routes/contracts.js';
import { registerEventRoutes } from './routes/events.js';
import { registerHealthRoutes } from './routes/health.js';
import { registerRelayerRoutes } from './routes/relayer.js';
import { registerTokenRoutes } from './routes/tokens.js';
import { registerWalletRoutes } from './routes/wallets.js';
import { manifest } from './lib/manifest.js';

const app = Fastify();

registerHealthRoutes(app);
registerWalletRoutes(app);
registerContractRoutes(app);
registerEventRoutes(app);
registerTokenRoutes(app);
if (manifest.api?.relayer) {
  registerRelayerRoutes(app);
}

app.listen({ port: Number(process.env.PORT ?? 3000), host: '0.0.0.0' }).catch((error) => {
  console.error(error);
  process.exit(1);
});
