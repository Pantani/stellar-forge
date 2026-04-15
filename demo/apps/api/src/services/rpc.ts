import { manifest } from '../lib/manifest.js';
import { loadApiConfig } from '../lib/config.js';

type RequestBody = Record<string, unknown>;

function asRecord(value: unknown): RequestBody {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as RequestBody;
  }
  return {};
}

function resolveArgs(body: RequestBody) {
  return asRecord(body.args ?? body.params ?? body);
}

export function readBody(value: unknown) {
  return asRecord(value);
}

export function contractPreviewTemplate(alias: string, fnName: string, body: RequestBody) {
  const config = loadApiConfig();
  return {
    alias,
    fn: fnName,
    mode: typeof body.mode === 'string' ? body.mode : 'auto',
    network: config.network,
    rpc_url: config.rpc_url,
    args: resolveArgs(body),
    preview: {
      strategy: 'simulate-or-build',
      command: `stellar contract invoke --id ${alias} --network ${config.network} --send no -- ${fnName}`,
    },
  };
}

export function contractTxTemplate(alias: string, fnName: string, body: RequestBody) {
  const config = loadApiConfig();
  const source =
    typeof body.source === 'string' && body.source.length > 0
      ? body.source
      : manifest.defaults.identity;
  return {
    alias,
    fn: fnName,
    source,
    network: config.network,
    rpc_url: config.rpc_url,
    args: resolveArgs(body),
    resource_fee: 'estimate-required',
    inclusion_fee: 'estimate-required',
    expected_signers: [source],
    sep7_uri: 'web+stellar:tx?xdr=<build-first>',
    build: {
      command: `stellar contract invoke --id ${alias} --source-account ${source} --network ${config.network} --build-only -- ${fnName}`,
    },
  };
}

export function tokenMetadataTemplate(alias: string) {
  return {
    token: alias,
    definition:
      manifest.tokens[alias as keyof typeof manifest.tokens] ?? null,
    network: loadApiConfig().network,
  };
}

export function tokenBalanceTemplate(alias: string, holder: string) {
  return {
    token: alias,
    holder,
    strategy: 'wallet-balance-lookup',
    command: `stellar forge token balance ${alias} --holder ${holder}`,
  };
}

export function tokenTrustTemplate(alias: string, body: RequestBody) {
  const wallet =
    typeof body.wallet === 'string' && body.wallet.length > 0
      ? body.wallet
      : manifest.defaults.identity;
  return {
    token: alias,
    wallet,
    build: {
      command: `stellar forge wallet trust ${wallet} ${alias}`,
    },
  };
}

export function tokenPaymentTemplate(alias: string, body: RequestBody) {
  const from =
    typeof body.from === 'string' && body.from.length > 0
      ? body.from
      : manifest.defaults.identity;
  return {
    token: alias,
    from,
    to: typeof body.to === 'string' ? body.to : null,
    amount: typeof body.amount === 'string' ? body.amount : null,
    sep7: body.sep7 === true,
    relayer: body.relayer === true,
    build: {
      command: `stellar forge wallet pay --from ${from} --to <destination> --asset ${alias} --amount <amount>`,
    },
  };
}

export function tokenMintTemplate(alias: string, body: RequestBody) {
  return {
    token: alias,
    to: typeof body.to === 'string' ? body.to : null,
    amount: typeof body.amount === 'string' ? body.amount : null,
    build: {
      command: `stellar forge token mint ${alias} --to <destination> --amount <amount>`,
    },
  };
}
