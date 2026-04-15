import { manifest } from '../../lib/manifest.js';

type ContractBody = Record<string, unknown>;

function asRecord(value: unknown): ContractBody {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as ContractBody;
  }
  return {};
}

function resolveArgs(body: ContractBody) {
  return asRecord(body.args ?? body.params ?? body);
}

export function resourceDefinition() {
  return {
    alias: 'app',
    path: 'contracts/app',
    template: 'basic',
    bindings: ["typescript"],
    typescript_binding: "packages/app-ts",
    preview_endpoint: '/contracts/app/call/:fn',
    tx_endpoint: '/contracts/app/tx/:fn',
    send_endpoint: null,
  };
}

export function preview(fnName: string, body: unknown = {}) {
  const payload = asRecord(body);
  return {
    ...resourceDefinition(),
    fn: fnName,
    mode: typeof payload.mode === 'string' ? payload.mode : 'auto',
    args: resolveArgs(payload),
  };
}

export function buildTx(fnName: string, body: unknown = {}) {
  const payload = asRecord(body);
  const source =
    typeof payload.source === 'string' && payload.source.length > 0
      ? payload.source
      : manifest.defaults.identity;
  return {
    ...resourceDefinition(),
    fn: fnName,
    source,
    args: resolveArgs(payload),
    expected_signers: [source],
    sep7_uri: 'web+stellar:tx?xdr=<build-first>',
  };
}

export function send(fnName: string, body: unknown = {}) {
  return {
    enabled: false,
    relay_endpoint: null,
    request: buildTx(fnName, body),
  };
}
