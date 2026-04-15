import { manifest } from './manifest.js';

export function loadApiConfig() {
  return {
    port: Number(process.env.PORT ?? 3000),
    network: process.env.STELLAR_NETWORK ?? manifest.defaults.network,
    rpc_url: process.env.STELLAR_RPC_URL ?? '',
    relayer_base_url: process.env.RELAYER_BASE_URL ?? '',
    relayer_api_key: process.env.RELAYER_API_KEY ?? '',
    relayer_submit_path: process.env.RELAYER_SUBMIT_PATH ?? '/transactions',
  };
}
