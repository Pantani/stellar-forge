use crate::cli::{InitArgs, ProjectTemplate};
use crate::model::{
    ApiConfig, ContractConfig, ContractInitConfig, DefaultsConfig, FrontendConfig, IdentityConfig,
    Lockfile, Manifest, NetworkConfig, ProjectConfig, ReleaseConfig, TokenConfig, WalletConfig,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct ContractTemplateFiles {
    pub lib_rs: String,
    pub test_rs: Option<String>,
    pub readme: String,
}

pub fn scaffold_manifest(args: &InitArgs) -> Manifest {
    let mut networks = BTreeMap::new();
    networks.insert(
        "local".to_string(),
        NetworkConfig {
            kind: "local".to_string(),
            rpc_url: "http://localhost:8000/rpc".to_string(),
            horizon_url: "http://localhost:8000".to_string(),
            network_passphrase: "Standalone Network ; February 2017".to_string(),
            allow_http: true,
            friendbot: true,
        },
    );
    networks.insert(
        "testnet".to_string(),
        NetworkConfig {
            kind: "testnet".to_string(),
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            horizon_url: "https://horizon-testnet.stellar.org".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            allow_http: false,
            friendbot: true,
        },
    );
    networks.insert(
        "futurenet".to_string(),
        NetworkConfig {
            kind: "futurenet".to_string(),
            rpc_url: "https://rpc-futurenet.stellar.org".to_string(),
            horizon_url: "https://horizon-futurenet.stellar.org".to_string(),
            network_passphrase: "Test SDF Future Network ; October 2022".to_string(),
            allow_http: false,
            friendbot: true,
        },
    );

    let mut identities = BTreeMap::new();
    identities.insert(
        "alice".to_string(),
        IdentityConfig {
            source: "stellar-cli".to_string(),
            name: "alice".to_string(),
        },
    );
    let mut wallets = BTreeMap::new();
    wallets.insert(
        "alice".to_string(),
        WalletConfig {
            kind: "classic".to_string(),
            identity: "alice".to_string(),
            controller_identity: None,
            mode: None,
            onboarding_app: None,
            policy_contract: None,
        },
    );

    let mut tokens = BTreeMap::new();
    let mut contracts = BTreeMap::new();
    let mut release = BTreeMap::new();
    let mut api = None;
    let mut frontend = None;

    match args.template {
        ProjectTemplate::ApiOnly => {
            api = Some(ApiConfig {
                enabled: true,
                openapi: true,
                ..ApiConfig::default()
            });
        }
        ProjectTemplate::MinimalContract => {
            contracts.insert("app".to_string(), sample_contract("app", "basic"));
        }
        ProjectTemplate::Fullstack => {
            contracts.insert("app".to_string(), sample_contract("app", "basic"));
            api = Some(ApiConfig {
                enabled: true,
                openapi: true,
                ..ApiConfig::default()
            });
            frontend = Some(FrontendConfig {
                enabled: true,
                framework: args.frontend.clone(),
            });
        }
        ProjectTemplate::IssuerWallet => {
            add_issuer_wallet(&mut identities, &mut wallets, &mut tokens);
            api = Some(ApiConfig {
                enabled: true,
                openapi: true,
                ..ApiConfig::default()
            });
            frontend = Some(FrontendConfig {
                enabled: true,
                framework: args.frontend.clone(),
            });
        }
        ProjectTemplate::MerchantCheckout => {
            add_issuer_wallet(&mut identities, &mut wallets, &mut tokens);
            api = Some(ApiConfig {
                enabled: true,
                openapi: true,
                ..ApiConfig::default()
            });
            frontend = Some(FrontendConfig {
                enabled: true,
                framework: args.frontend.clone(),
            });
        }
        ProjectTemplate::RewardsLoyalty => {
            add_issuer_wallet(&mut identities, &mut wallets, &mut tokens);
            contracts.insert(
                "rewards".to_string(),
                ContractConfig {
                    path: "contracts/rewards".to_string(),
                    alias: "rewards".to_string(),
                    template: "rewards".to_string(),
                    bindings: vec!["typescript".to_string()],
                    deploy_on: vec!["local".to_string(), "testnet".to_string()],
                    init: Some(ContractInitConfig {
                        fn_name: "init".to_string(),
                        args: BTreeMap::from([
                            ("admin".to_string(), "@identity:issuer".to_string()),
                            ("token".to_string(), "@token:points:sac".to_string()),
                        ]),
                    }),
                },
            );
            api = Some(ApiConfig {
                enabled: true,
                openapi: true,
                ..ApiConfig::default()
            });
            frontend = Some(FrontendConfig {
                enabled: true,
                framework: args.frontend.clone(),
            });
            release.insert(
                "testnet".to_string(),
                ReleaseConfig {
                    deploy_contracts: vec!["rewards".to_string()],
                    deploy_tokens: vec!["points".to_string()],
                    generate_env: true,
                },
            );
        }
        ProjectTemplate::MultiContract => {
            contracts.insert("app".to_string(), sample_contract("app", "basic"));
            contracts.insert("escrow".to_string(), sample_contract("escrow", "escrow"));
            api = Some(ApiConfig {
                enabled: true,
                openapi: true,
                ..ApiConfig::default()
            });
        }
    }

    if args.api && !args.no_api && api.is_none() {
        api = Some(ApiConfig {
            enabled: true,
            openapi: true,
            ..ApiConfig::default()
        });
    }
    if matches!(
        args.template,
        ProjectTemplate::Fullstack
            | ProjectTemplate::MerchantCheckout
            | ProjectTemplate::RewardsLoyalty
            | ProjectTemplate::IssuerWallet
    ) && frontend.is_none()
    {
        frontend = Some(FrontendConfig {
            enabled: true,
            framework: args.frontend.clone(),
        });
    }

    Manifest {
        project: ProjectConfig {
            name: args.name.clone(),
            slug: slugify(&args.name),
            version: "0.1.0".to_string(),
            package_manager: args.package_manager.clone(),
        },
        defaults: DefaultsConfig {
            network: args.network.clone(),
            identity: "alice".to_string(),
            output: "human".to_string(),
        },
        networks,
        identities,
        wallets,
        tokens,
        contracts,
        api,
        frontend,
        release,
    }
}

pub fn env_example(manifest: &Manifest) -> String {
    let network = manifest.defaults.network.clone();
    let rpc_url = manifest
        .networks
        .get(&network)
        .map(|network| network.rpc_url.clone())
        .unwrap_or_default();
    format!(
        "STELLAR_NETWORK={network}\nSTELLAR_RPC_URL={rpc_url}\nSTELLAR_DEFAULT_IDENTITY={}\n",
        manifest.defaults.identity
    )
}

pub fn readme(manifest: &Manifest) -> String {
    let api = manifest.api.as_ref().is_some_and(|api| api.enabled);
    let web = manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled);
    let helper_lines = if api {
        "- `node scripts/doctor.mjs`\n- `node scripts/reseed.mjs`\n- `node scripts/release.mjs --plan`\n- `node workers/events/ingest-events.mjs <resource> --once`\n"
    } else {
        "- `node scripts/doctor.mjs`\n- `node scripts/reseed.mjs`\n- `node scripts/release.mjs --plan`\n"
    };
    format!(
        "# {}\n\nGenerated by `stellar forge init`.\n\n## Next steps\n\n1. Run `stellar forge doctor`.\n2. Run `stellar forge dev up`.\n3. Build and deploy contracts with `stellar forge contract build` and `stellar forge release deploy {}`.\n\n## Repo helpers\n\n{}\n## Enabled modules\n\n- API: {}\n- Frontend: {}\n- Contracts: {}\n",
        manifest.project.name,
        manifest.defaults.network,
        helper_lines,
        if api { "yes" } else { "no" },
        if web { "yes" } else { "no" },
        manifest.contracts.len()
    )
}

pub fn gitignore() -> &'static str {
    "/target\n/node_modules\n/.env.generated\n/dist\n/.DS_Store\n"
}

pub fn project_doctor_script() -> &'static str {
    r####"#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function loadDefaults() {
  const defaults = {};
  for (const relative of ['.env.generated', '.env.example']) {
    const file = resolve(root, relative);
    if (!existsSync(file)) {
      continue;
    }
    for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      const separator = trimmed.indexOf('=');
      if (separator === -1) {
        continue;
      }
      const key = trimmed.slice(0, separator).trim();
      const value = trimmed.slice(separator + 1).trim();
      if (value && !(key in defaults)) {
        defaults[key] = value;
      }
    }
  }
  return defaults;
}

function runForge(args) {
  const command = process.env.STELLAR_FORGE_BIN || 'stellar-forge';
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
  if (result.error) {
    if (result.error.code === 'ENOENT') {
      console.error(`Could not find ${command} on PATH.`);
      console.error('Set STELLAR_FORGE_BIN to override the executable name.');
      process.exit(1);
    }
    throw result.error;
  }
  process.exit(result.status ?? 1);
}

const defaults = loadDefaults();
const network = defaults.STELLAR_NETWORK || 'testnet';
const userArgs = process.argv.slice(2);
const hasNetworkFlag = userArgs.some((arg) => arg === '--network' || arg.startsWith('--network='));
const args = hasNetworkFlag
  ? ['doctor', ...userArgs]
  : ['--network', network, 'doctor', ...userArgs];

runForge(args);
"####
}

pub fn project_reseed_script() -> &'static str {
    r####"#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function loadDefaults() {
  const defaults = {};
  for (const relative of ['.env.generated', '.env.example']) {
    const file = resolve(root, relative);
    if (!existsSync(file)) {
      continue;
    }
    for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      const separator = trimmed.indexOf('=');
      if (separator === -1) {
        continue;
      }
      const key = trimmed.slice(0, separator).trim();
      const value = trimmed.slice(separator + 1).trim();
      if (value && !(key in defaults)) {
        defaults[key] = value;
      }
    }
  }
  return defaults;
}

function runForge(args) {
  const command = process.env.STELLAR_FORGE_BIN || 'stellar-forge';
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
  if (result.error) {
    if (result.error.code === 'ENOENT') {
      console.error(`Could not find ${command} on PATH.`);
      console.error('Set STELLAR_FORGE_BIN to override the executable name.');
      process.exit(1);
    }
    throw result.error;
  }
  process.exit(result.status ?? 1);
}

const defaults = loadDefaults();
const network = defaults.STELLAR_NETWORK || 'testnet';
const userArgs = process.argv.slice(2);
const hasNetworkFlag = userArgs.some((arg) => arg === '--network' || arg.startsWith('--network='));
const args = hasNetworkFlag
  ? ['dev', 'reseed', ...userArgs]
  : ['--network', network, 'dev', 'reseed', ...userArgs];

runForge(args);
"####
}

pub fn project_release_script() -> &'static str {
    r####"#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function loadDefaults() {
  const defaults = {};
  for (const relative of ['.env.generated', '.env.example']) {
    const file = resolve(root, relative);
    if (!existsSync(file)) {
      continue;
    }
    for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      const separator = trimmed.indexOf('=');
      if (separator === -1) {
        continue;
      }
      const key = trimmed.slice(0, separator).trim();
      const value = trimmed.slice(separator + 1).trim();
      if (value && !(key in defaults)) {
        defaults[key] = value;
      }
    }
  }
  return defaults;
}

function runForge(args) {
  const command = process.env.STELLAR_FORGE_BIN || 'stellar-forge';
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
  if (result.error) {
    if (result.error.code === 'ENOENT') {
      console.error(`Could not find ${command} on PATH.`);
      console.error('Set STELLAR_FORGE_BIN to override the executable name.');
      process.exit(1);
    }
    throw result.error;
  }
  process.exit(result.status ?? 1);
}

const defaults = loadDefaults();
const rawArgs = process.argv.slice(2);
let mode = 'deploy';
let targetEnv = defaults.STELLAR_NETWORK || 'testnet';
let envExplicit = false;
const forwarded = [];

for (const arg of rawArgs) {
  if (arg === '--plan') {
    mode = 'plan';
    continue;
  }
  if (arg === '--verify') {
    mode = 'verify';
    continue;
  }
  if (arg === '--env-export') {
    mode = 'env-export';
    continue;
  }
  if (arg === '--aliases-sync') {
    mode = 'aliases-sync';
    continue;
  }
  if (!arg.startsWith('-') && !envExplicit) {
    targetEnv = arg;
    envExplicit = true;
    continue;
  }
  forwarded.push(arg);
}

const args =
  mode === 'plan'
    ? ['release', 'plan', targetEnv, ...forwarded]
    : mode === 'verify'
      ? ['release', 'verify', targetEnv, ...forwarded]
      : mode === 'env-export'
        ? ['release', 'env', 'export', targetEnv, ...forwarded]
        : mode === 'aliases-sync'
          ? ['release', 'aliases', 'sync', targetEnv, ...forwarded]
          : ['release', 'deploy', targetEnv, ...forwarded];

runForge(args);
"####
}

pub fn api_server(_manifest: &Manifest) -> String {
    "import Fastify from 'fastify';\nimport { registerContractRoutes } from './routes/contracts.js';\nimport { registerEventRoutes } from './routes/events.js';\nimport { registerHealthRoutes } from './routes/health.js';\nimport { registerRelayerRoutes } from './routes/relayer.js';\nimport { registerTokenRoutes } from './routes/tokens.js';\nimport { registerWalletRoutes } from './routes/wallets.js';\nimport { manifest } from './lib/manifest.js';\n\nconst app = Fastify();\n\nregisterHealthRoutes(app);\nregisterWalletRoutes(app);\nregisterContractRoutes(app);\nregisterEventRoutes(app);\nregisterTokenRoutes(app);\nif (manifest.api?.relayer) {\n  registerRelayerRoutes(app);\n}\n\napp.listen({ port: Number(process.env.PORT ?? 3000), host: '0.0.0.0' }).catch((error) => {\n  console.error(error);\n  process.exit(1);\n});\n".to_string()
}

pub fn api_package_json() -> &'static str {
    "{\n  \"name\": \"@stellar-forge/api\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {\n    \"dev\": \"tsx watch src/server.ts\",\n    \"start\": \"tsx src/server.ts\",\n    \"events:ingest\": \"tsx src/workers/ingest-events.ts\"\n  },\n  \"dependencies\": {\n    \"better-sqlite3\": \"^12.4.1\",\n    \"dotenv\": \"^16.6.1\",\n    \"fastify\": \"^5.0.0\"\n  },\n  \"devDependencies\": {\n    \"@types/node\": \"^24.6.0\",\n    \"tsx\": \"^4.20.6\",\n    \"typescript\": \"^5.9.3\"\n  }\n}\n"
}

pub fn api_tsconfig() -> &'static str {
    "{\n  \"compilerOptions\": {\n    \"target\": \"ES2022\",\n    \"module\": \"NodeNext\",\n    \"moduleResolution\": \"NodeNext\",\n    \"strict\": true,\n    \"esModuleInterop\": true,\n    \"skipLibCheck\": true,\n    \"types\": [\"node\"]\n  },\n  \"include\": [\"src/**/*.ts\"]\n}\n"
}

pub fn api_env_example(manifest: &Manifest) -> String {
    format!(
        "{}PORT=3000\nSTELLAR_EVENTS_DB_PATH=./db/events.sqlite\nSTELLAR_EVENTS_POLL_INTERVAL_MS=5000\nSTELLAR_EVENTS_BATCH_SIZE=200\nSTELLAR_EVENTS_START_LEDGER=\nSTELLAR_EVENTS_RESOURCES=\nSTELLAR_EVENTS_TOPICS=\nSTELLAR_EVENTS_TYPE=all\nSTELLAR_EVENTS_RETENTION_DAYS=7\n",
        env_example(manifest)
    )
}

pub fn api_config() -> &'static str {
    r####"import { manifest } from './manifest.js';

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
"####
}

pub fn api_errors() -> &'static str {
    r####"export class HttpError extends Error {
  statusCode: number;
  detail: unknown;

  constructor(statusCode: number, message: string, detail: unknown = null) {
    super(message);
    this.statusCode = statusCode;
    this.detail = detail;
  }
}

export function requireConfigured(value: string, name: string) {
  if (!value || value.trim().length === 0) {
    throw new HttpError(503, `${name} is not configured`);
  }
  return value;
}
"####
}

pub fn api_rpc_service() -> &'static str {
    r####"import { manifest } from '../lib/manifest.js';
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
"####
}

pub fn api_health_routes() -> &'static str {
    r####"import type { FastifyInstance } from 'fastify';
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
"####
}

pub fn api_wallet_routes() -> &'static str {
    r####"import type { FastifyInstance } from 'fastify';
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
"####
}

pub fn api_contract_resource_service(
    name: &str,
    contract: &ContractConfig,
    relayer_enabled: bool,
) -> String {
    let bindings = serde_json::to_string(&contract.bindings).expect("bindings should serialize");
    let typescript_binding = if contract
        .bindings
        .iter()
        .any(|binding| matches!(binding.as_str(), "typescript" | "javascript"))
    {
        Some(format!("packages/{name}-ts"))
    } else {
        None
    };
    let typescript_binding =
        serde_json::to_string(&typescript_binding).expect("binding path should serialize");
    let send_endpoint = if relayer_enabled {
        format!("'/contracts/{name}/send/:fn'")
    } else {
        "null".to_string()
    };
    let relay_endpoint = if relayer_enabled {
        "'/relayer/submit'"
    } else {
        "null"
    };
    format!(
        "import {{ manifest }} from '../../lib/manifest.js';\n\ntype ContractBody = Record<string, unknown>;\n\nfunction asRecord(value: unknown): ContractBody {{\n  if (value && typeof value === 'object' && !Array.isArray(value)) {{\n    return value as ContractBody;\n  }}\n  return {{}};\n}}\n\nfunction resolveArgs(body: ContractBody) {{\n  return asRecord(body.args ?? body.params ?? body);\n}}\n\nexport function resourceDefinition() {{\n  return {{\n    alias: '{name}',\n    path: '{path}',\n    template: '{template}',\n    bindings: {bindings},\n    typescript_binding: {typescript_binding},\n    preview_endpoint: '/contracts/{name}/call/:fn',\n    tx_endpoint: '/contracts/{name}/tx/:fn',\n    send_endpoint: {send_endpoint},\n  }};\n}}\n\nexport function preview(fnName: string, body: unknown = {{}}) {{\n  const payload = asRecord(body);\n  return {{\n    ...resourceDefinition(),\n    fn: fnName,\n    mode: typeof payload.mode === 'string' ? payload.mode : 'auto',\n    args: resolveArgs(payload),\n  }};\n}}\n\nexport function buildTx(fnName: string, body: unknown = {{}}) {{\n  const payload = asRecord(body);\n  const source =\n    typeof payload.source === 'string' && payload.source.length > 0\n      ? payload.source\n      : manifest.defaults.identity;\n  return {{\n    ...resourceDefinition(),\n    fn: fnName,\n    source,\n    args: resolveArgs(payload),\n    expected_signers: [source],\n    sep7_uri: 'web+stellar:tx?xdr=<build-first>',\n  }};\n}}\n\nexport function send(fnName: string, body: unknown = {{}}) {{\n  return {{\n    enabled: {relayer_enabled},\n    relay_endpoint: {relay_endpoint},\n    request: buildTx(fnName, body),\n  }};\n}}\n",
        path = contract.path,
        template = contract.template,
    )
}

pub fn api_token_resource_service(name: &str, token: &TokenConfig) -> String {
    let builders = {
        let mut builders = vec![
            "balance".to_string(),
            "payment".to_string(),
            "mint".to_string(),
        ];
        if token.kind != "contract" {
            builders.push("trust".to_string());
        }
        if token.with_sac {
            builders.push("sac_transfer".to_string());
        }
        serde_json::to_string(&builders).expect("builders should serialize")
    };
    let trust_endpoint = if token.kind == "contract" {
        "null".to_string()
    } else {
        format!("'/tokens/{name}/trust'")
    };
    let trust_supported = if token.kind == "contract" {
        "false"
    } else {
        "true"
    };
    format!(
        "import {{ manifest }} from '../../lib/manifest.js';\n\ntype TokenBody = Record<string, unknown>;\n\nfunction asRecord(value: unknown): TokenBody {{\n  if (value && typeof value === 'object' && !Array.isArray(value)) {{\n    return value as TokenBody;\n  }}\n  return {{}};\n}}\n\nexport function resourceDefinition() {{\n  return {{\n    token: '{name}',\n    kind: '{kind}',\n    code: '{code}',\n    with_sac: {with_sac},\n    builders: {builders},\n    metadata_endpoint: '/tokens/{name}',\n    balance_endpoint: '/tokens/{name}/balances/:holder',\n    trust_endpoint: {trust_endpoint},\n    payment_endpoint: '/tokens/{name}/payment',\n    mint_endpoint: '/tokens/{name}/mint',\n  }};\n}}\n\nexport function metadata() {{\n  return {{\n    ...resourceDefinition(),\n    definition: manifest.tokens['{name}' as keyof typeof manifest.tokens] ?? null,\n  }};\n}}\n\nexport function balance(holder: string) {{\n  return {{\n    ...resourceDefinition(),\n    holder,\n    command: `stellar forge token balance {name} --holder ${{holder}}`,\n  }};\n}}\n\nexport function trust(body: unknown = {{}}) {{\n  const payload = asRecord(body);\n  const wallet =\n    typeof payload.wallet === 'string' && payload.wallet.length > 0\n      ? payload.wallet\n      : manifest.defaults.identity;\n  return {{\n    ...resourceDefinition(),\n    wallet,\n    supported: {trust_supported},\n    build: {trust_build},\n  }};\n}}\n\nexport function payment(body: unknown = {{}}) {{\n  const payload = asRecord(body);\n  const from =\n    typeof payload.from === 'string' && payload.from.length > 0\n      ? payload.from\n      : manifest.defaults.identity;\n  return {{\n    ...resourceDefinition(),\n    from,\n    to: typeof payload.to === 'string' ? payload.to : null,\n    amount: typeof payload.amount === 'string' ? payload.amount : null,\n    relayer: payload.relayer === true,\n    sep7: payload.sep7 === true,\n    build: `stellar forge wallet pay --from ${{from}} --to <destination> --asset {name} --amount <amount>`,\n  }};\n}}\n\nexport function mint(body: unknown = {{}}) {{\n  const payload = asRecord(body);\n  return {{\n    ...resourceDefinition(),\n    to: typeof payload.to === 'string' ? payload.to : null,\n    amount: typeof payload.amount === 'string' ? payload.amount : null,\n    build: `stellar forge token mint {name} --to <destination> --amount <amount>`,\n  }};\n}}\n",
        kind = token.kind,
        code = if token.code.is_empty() {
            "XLM".to_string()
        } else {
            token.code.clone()
        },
        with_sac = token.with_sac,
        trust_build = if token.kind == "contract" {
            "null".to_string()
        } else {
            format!("`stellar forge wallet trust ${{wallet}} {name}`")
        },
    )
}

pub fn api_relayer_service() -> &'static str {
    r####"import { loadApiConfig } from '../lib/config.js';
import { HttpError, requireConfigured } from '../lib/errors.js';

type RelayerPayload = Record<string, unknown>;

export function relayerStatus() {
  const config = loadApiConfig();
  return {
    configured:
      config.relayer_base_url.length > 0 &&
      config.relayer_api_key.length > 0,
    relayer_base_url: config.relayer_base_url || null,
    relayer_submit_path: config.relayer_submit_path,
  };
}

export async function submitSponsoredTransaction(payload: RelayerPayload) {
  const config = loadApiConfig();
  const baseUrl = requireConfigured(config.relayer_base_url, 'RELAYER_BASE_URL');
  const apiKey = requireConfigured(config.relayer_api_key, 'RELAYER_API_KEY');
  const url = new URL(config.relayer_submit_path, baseUrl);

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify(payload),
  });

  const rawBody = await response.text();
  let parsed: unknown = null;
  if (rawBody.length > 0) {
    try {
      parsed = JSON.parse(rawBody);
    } catch {
      parsed = { raw: rawBody };
    }
  }

  if (!response.ok) {
    throw new HttpError(
      response.status,
      `relayer request failed with status ${response.status}`,
      parsed,
    );
  }

  return {
    accepted: true,
    status_code: response.status,
    upstream_url: url.toString(),
    upstream_response: parsed,
  };
}
"####
}

pub fn api_relayer_routes() -> &'static str {
    r####"import type { FastifyInstance } from 'fastify';
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
"####
}

pub fn api_events_schema() -> &'static str {
    "create table if not exists cursors (\n  name text primary key,\n  resource_kind text not null,\n  resource_name text not null,\n  cursor text,\n  last_ledger integer,\n  updated_at text not null\n);\ncreate table if not exists events (\n  id integer primary key autoincrement,\n  external_id text not null unique,\n  cursor_name text not null,\n  cursor text,\n  resource_kind text not null,\n  resource_name text not null,\n  contract_id text not null,\n  event_type text not null,\n  topic text not null,\n  payload text not null,\n  tx_hash text,\n  ledger integer,\n  observed_at text not null\n);\ncreate index if not exists idx_events_cursor_name on events (cursor_name, ledger desc);\ncreate index if not exists idx_events_contract_id on events (contract_id, ledger desc);\n"
}

pub fn api_events_store() -> &'static str {
    r####"import Database from 'better-sqlite3';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export type EventCursorRow = {
  name: string;
  resource_kind: string;
  resource_name: string;
  cursor: string | null;
  last_ledger: number | null;
  updated_at: string;
};

export type NormalizedEvent = {
  external_id: string;
  cursor_name: string;
  cursor: string | null;
  resource_kind: string;
  resource_name: string;
  contract_id: string;
  event_type: string;
  topic: string;
  payload: string;
  tx_hash: string | null;
  ledger: number | null;
  observed_at: string;
};

export type EventWorkerConfig = {
  batch_size: number;
  poll_interval_ms: number;
  start_ledger: number | null;
  resources: string[];
  topic_filters: string[];
  event_type: 'all' | 'contract' | 'system';
  retention_days: number | null;
};

const apiRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const projectRoot = path.resolve(apiRoot, '..', '..');
const fallbackSchema = `create table if not exists cursors (
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
`;

function resolvePath(base: string, candidate: string) {
  return path.isAbsolute(candidate) ? candidate : path.resolve(base, candidate);
}

export function resolveEventPaths() {
  const dbPath = resolvePath(
    apiRoot,
    process.env.STELLAR_EVENTS_DB_PATH ?? './db/events.sqlite',
  );
  const schemaPath = resolvePath(
    apiRoot,
    process.env.STELLAR_EVENTS_SCHEMA_PATH ?? './db/schema.sql',
  );
  const cursorSnapshotPath = resolvePath(
    apiRoot,
    process.env.STELLAR_EVENTS_CURSOR_FILE ?? '../../workers/events/cursors.json',
  );
  return { dbPath, schemaPath, cursorSnapshotPath };
}

function parsePositiveInteger(value: string | undefined) {
  if (!value || value.length === 0) {
    return null;
  }
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }
  return parsed;
}

function parseList(value: string | undefined) {
  if (!value) {
    return [];
  }
  return value
    .split(/\r?\n|,/)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function parseTopicList(value: string | undefined) {
  if (!value) {
    return [];
  }
  return value
    .split(/\r?\n|;/)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function parseEventType(value: string | undefined): EventWorkerConfig['event_type'] {
  if (value === 'contract' || value === 'system') {
    return value;
  }
  return 'all';
}

export function resolveEventWorkerConfig(): EventWorkerConfig {
  return {
    batch_size: parsePositiveInteger(process.env.STELLAR_EVENTS_BATCH_SIZE) ?? 200,
    poll_interval_ms:
      parsePositiveInteger(process.env.STELLAR_EVENTS_POLL_INTERVAL_MS) ?? 5000,
    start_ledger: parsePositiveInteger(process.env.STELLAR_EVENTS_START_LEDGER),
    resources: parseList(process.env.STELLAR_EVENTS_RESOURCES),
    topic_filters: parseTopicList(process.env.STELLAR_EVENTS_TOPICS),
    event_type: parseEventType(process.env.STELLAR_EVENTS_TYPE),
    retention_days: parsePositiveInteger(process.env.STELLAR_EVENTS_RETENTION_DAYS),
  };
}

function ensureSchema(db: Database.Database) {
  const { schemaPath } = resolveEventPaths();
  const schema = fs.existsSync(schemaPath)
    ? fs.readFileSync(schemaPath, 'utf8')
    : fallbackSchema;
  db.exec(schema);
}

function withStore<T>(store: Database.Database | undefined, fn: (db: Database.Database) => T): T {
  const db = store ?? openEventStore();
  try {
    return fn(db);
  } finally {
    if (!store) {
      db.close();
    }
  }
}

export function openEventStore() {
  const { dbPath } = resolveEventPaths();
  fs.mkdirSync(path.dirname(dbPath), { recursive: true });
  const db = new Database(dbPath);
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  ensureSchema(db);
  return db;
}

export function loadCursor(name: string, store?: Database.Database): EventCursorRow | null {
  return withStore(store, (db) => {
    const row = db
      .prepare(
        `select name, resource_kind, resource_name, cursor, last_ledger, updated_at
         from cursors
         where name = ?`,
      )
      .get(name) as EventCursorRow | undefined;
    return row ?? null;
  });
}

export function upsertCursor(row: EventCursorRow, store?: Database.Database) {
  return withStore(store, (db) => {
    db.prepare(
      `insert into cursors (name, resource_kind, resource_name, cursor, last_ledger, updated_at)
       values (@name, @resource_kind, @resource_name, @cursor, @last_ledger, @updated_at)
       on conflict(name) do update set
         resource_kind = excluded.resource_kind,
         resource_name = excluded.resource_name,
         cursor = excluded.cursor,
         last_ledger = excluded.last_ledger,
         updated_at = excluded.updated_at`,
    ).run(row);
  });
}

export function insertEvent(event: NormalizedEvent, store?: Database.Database) {
  return withStore(store, (db) => {
    db.prepare(
      `insert or ignore into events (
        external_id,
        cursor_name,
        cursor,
        resource_kind,
        resource_name,
        contract_id,
        event_type,
        topic,
        payload,
        tx_hash,
        ledger,
        observed_at
      ) values (
        @external_id,
        @cursor_name,
        @cursor,
        @resource_kind,
        @resource_name,
        @contract_id,
        @event_type,
        @topic,
        @payload,
        @tx_hash,
        @ledger,
        @observed_at
      )`,
    ).run(event);
  });
}

export function listEventCursors(store?: Database.Database): EventCursorRow[] {
  const { dbPath } = resolveEventPaths();
  if (!store && !fs.existsSync(dbPath)) {
    return [];
  }
  return withStore(store, (db) => {
    return db
      .prepare(
        `select name, resource_kind, resource_name, cursor, last_ledger, updated_at
         from cursors
         order by name asc`,
      )
      .all() as EventCursorRow[];
  });
}

export function getEventStatus(store?: Database.Database) {
  const { dbPath } = resolveEventPaths();
  if (!store && !fs.existsSync(dbPath)) {
    return {
      total_events: 0,
      latest_ledger: null,
      latest_observed_at: null,
      cursor_count: 0,
    };
  }
  return withStore(store, (db) => {
    const summary = db
      .prepare(
        `select
           count(*) as total_events,
           max(ledger) as latest_ledger,
           max(observed_at) as latest_observed_at
         from events`,
      )
      .get() as {
      total_events: number;
      latest_ledger: number | null;
      latest_observed_at: string | null;
    };
    const cursorSummary = db
      .prepare(`select count(*) as cursor_count from cursors`)
      .get() as { cursor_count: number };
    return {
      total_events: summary.total_events ?? 0,
      latest_ledger: summary.latest_ledger ?? null,
      latest_observed_at: summary.latest_observed_at ?? null,
      cursor_count: cursorSummary.cursor_count ?? 0,
    };
  });
}

export function syncCursorSnapshot(store?: Database.Database) {
  const rows = listEventCursors(store);
  const { cursorSnapshotPath } = resolveEventPaths();
  fs.mkdirSync(path.dirname(cursorSnapshotPath), { recursive: true });
  const cursors = Object.fromEntries(
    rows.map((row) => [
      row.name,
      {
        cursor: row.cursor,
        last_ledger: row.last_ledger,
        updated_at: row.updated_at,
      },
    ]),
  );
  fs.writeFileSync(
    cursorSnapshotPath,
    JSON.stringify({ cursors }, null, 2) + '\n',
    'utf8',
  );
  return cursorSnapshotPath;
}
"####
}

pub fn api_events_worker() -> &'static str {
    r####"import { execFile } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { config as loadDotenv } from 'dotenv';
import { manifest } from '../lib/manifest.js';
import {
  insertEvent,
  loadCursor,
  openEventStore,
  resolveEventWorkerConfig,
  syncCursorSnapshot,
  upsertCursor,
} from '../lib/events-store.js';

type TrackedResource = {
  kind: 'contract' | 'token';
  name: string;
  contractId: string;
};

type RawEvent = Record<string, unknown>;

const execFileAsync = promisify(execFile);
const workerFile = fileURLToPath(import.meta.url);
const apiRoot = path.resolve(path.dirname(workerFile), '../..');
const projectRoot = path.resolve(apiRoot, '..', '..');

function loadForgeEnv() {
  const candidates = [
    path.join(apiRoot, '.env'),
    path.join(apiRoot, '.env.local'),
    path.join(projectRoot, '.env'),
    path.join(projectRoot, '.env.local'),
    path.join(projectRoot, '.env.generated'),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      loadDotenv({ path: candidate, override: false });
    }
  }
}

function shouty(value: string) {
  return value.replace(/[^A-Za-z0-9]+/g, '_').replace(/^_+|_+$/g, '').toUpperCase();
}

function nonEmpty(values: Array<string | undefined>) {
  return values.find((value) => typeof value === 'string' && value.length > 0);
}

function deployedResourceId(kind: 'contract' | 'token', name: string) {
  const key = shouty(name);
  if (kind === 'contract') {
    return nonEmpty([
      process.env[`PUBLIC_${key}_CONTRACT_ID`],
      process.env[`STELLAR_${key}_CONTRACT_ID`],
    ]);
  }
  return nonEmpty([
    process.env[`PUBLIC_${key}_SAC_ID`],
    process.env[`STELLAR_${key}_SAC_ID`],
    process.env[`PUBLIC_${key}_TOKEN_ID`],
    process.env[`STELLAR_${key}_TOKEN_ID`],
  ]);
}

function trackedResources(resourceFilters: string[]): TrackedResource[] {
  const allow = new Set(resourceFilters);
  const matches = (kind: 'contract' | 'token', name: string) =>
    allow.size === 0 || allow.has(name) || allow.has(`${kind}:${name}`);

  const contracts = Object.keys(manifest.contracts)
    .filter((name) => matches('contract', name))
    .map((name) => ({
      kind: 'contract' as const,
      name,
      contractId: deployedResourceId('contract', name) ?? '',
    }))
    .filter((resource) => resource.contractId.length > 0);
  const tokens = Object.keys(manifest.tokens)
    .filter((name) => matches('token', name))
    .map((name) => ({
      kind: 'token' as const,
      name,
      contractId: deployedResourceId('token', name) ?? '',
    }))
    .filter((resource) => resource.contractId.length > 0);
  return [...contracts, ...tokens];
}

function activeNetwork() {
  return process.env.STELLAR_NETWORK ?? manifest.defaults.network;
}

function looksLikeXdrSegment(value: string) {
  if (value.length < 8 || value.length % 4 !== 0) {
    return false;
  }
  try {
    const decoded = Buffer.from(value, 'base64');
    return decoded.length >= 8 && decoded.length % 4 === 0;
  } catch {
    return false;
  }
}

function looksLikeSymbol(value: string) {
  return /^[A-Za-z0-9_]{1,32}$/.test(value);
}

function encodeScValWithPayload(tag: number, payload: Buffer) {
  const header = Buffer.alloc(4);
  header.writeUInt32BE(tag, 0);
  return Buffer.concat([header, payload]).toString('base64');
}

function encodeScValStringLike(tag: number, value: string) {
  const bytes = Buffer.from(value, 'utf8');
  const size = Buffer.alloc(4);
  size.writeUInt32BE(bytes.length, 0);
  const padding = Buffer.alloc((4 - (bytes.length % 4)) % 4);
  return encodeScValWithPayload(tag, Buffer.concat([size, bytes, padding]));
}

function encodeTopicSegment(segment: string) {
  if (segment === '*' || segment === '**' || looksLikeXdrSegment(segment)) {
    return segment;
  }

  if (segment.startsWith('sym:') || segment.startsWith('symbol:')) {
    const value = segment.startsWith('sym:')
      ? segment.slice('sym:'.length)
      : segment.slice('symbol:'.length);
    if (!looksLikeSymbol(value)) {
      throw new Error(`invalid symbol topic segment \`${value}\``);
    }
    return encodeScValStringLike(15, value);
  }
  if (segment.startsWith('str:') || segment.startsWith('string:')) {
    const value = segment.startsWith('str:')
      ? segment.slice('str:'.length)
      : segment.slice('string:'.length);
    return encodeScValStringLike(14, value);
  }
  if (segment.startsWith('bool:')) {
    const value = segment.slice('bool:'.length);
    if (value !== 'true' && value !== 'false') {
      throw new Error(`invalid bool topic segment \`${value}\``);
    }
    return encodeScValWithPayload(0, Buffer.from([0, 0, 0, value === 'true' ? 1 : 0]));
  }
  if (segment.startsWith('u32:')) {
    const value = Number.parseInt(segment.slice('u32:'.length), 10);
    if (!Number.isFinite(value) || value < 0) {
      throw new Error(`invalid u32 topic segment \`${segment}\``);
    }
    const payload = Buffer.alloc(4);
    payload.writeUInt32BE(value, 0);
    return encodeScValWithPayload(3, payload);
  }
  if (segment.startsWith('i32:')) {
    const value = Number.parseInt(segment.slice('i32:'.length), 10);
    if (!Number.isFinite(value)) {
      throw new Error(`invalid i32 topic segment \`${segment}\``);
    }
    const payload = Buffer.alloc(4);
    payload.writeInt32BE(value, 0);
    return encodeScValWithPayload(4, payload);
  }
  if (segment.startsWith('u64:')) {
    const payload = Buffer.alloc(8);
    payload.writeBigUInt64BE(BigInt(segment.slice('u64:'.length)), 0);
    return encodeScValWithPayload(5, payload);
  }
  if (segment.startsWith('i64:')) {
    const payload = Buffer.alloc(8);
    payload.writeBigInt64BE(BigInt(segment.slice('i64:'.length)), 0);
    return encodeScValWithPayload(6, payload);
  }

  if (looksLikeSymbol(segment)) {
    return encodeScValStringLike(15, segment);
  }
  return encodeScValStringLike(14, segment);
}

function normalizeTopicFilter(filter: string) {
  const segments = filter
    .split(',')
    .map((segment) => segment.trim())
    .filter((segment) => segment.length > 0);
  if (segments.length === 0) {
    throw new Error(`topic filter \`${filter}\` is empty`);
  }
  if (segments.length > 5) {
    throw new Error(
      `topic filter \`${filter}\` has too many segments; use up to 4 plus an optional trailing **`,
    );
  }
  const deepWildcard = segments.indexOf('**');
  if (deepWildcard >= 0 && deepWildcard !== segments.length - 1) {
    throw new Error(
      `topic filter \`${filter}\` uses ** before the end; it must be the last segment`,
    );
  }
  return segments.map(encodeTopicSegment).join(',');
}

function extractEvents(payload: unknown): RawEvent[] {
  if (Array.isArray(payload)) {
    return payload.filter((entry): entry is RawEvent => typeof entry === 'object' && entry !== null);
  }
  if (payload && typeof payload === 'object') {
    const record = payload as Record<string, unknown>;
    if (Array.isArray(record.events)) {
      return record.events.filter((entry): entry is RawEvent => typeof entry === 'object' && entry !== null);
    }
    if (Array.isArray(record.records)) {
      return record.records.filter((entry): entry is RawEvent => typeof entry === 'object' && entry !== null);
    }
    if (
      record.result &&
      typeof record.result === 'object' &&
      Array.isArray((record.result as Record<string, unknown>).events)
    ) {
      return ((record.result as Record<string, unknown>).events as unknown[]).filter(
        (entry): entry is RawEvent => typeof entry === 'object' && entry !== null,
      );
    }
    if (
      record.result &&
      typeof record.result === 'object' &&
      Array.isArray((record.result as Record<string, unknown>).records)
    ) {
      return ((record.result as Record<string, unknown>).records as unknown[]).filter(
        (entry): entry is RawEvent => typeof entry === 'object' && entry !== null,
      );
    }
  }
  return [];
}

function eventString(event: RawEvent, keys: string[]) {
  for (const key of keys) {
    const value = event[key];
    if (typeof value === 'string' && value.length > 0) {
      return value;
    }
  }
  return undefined;
}

function eventNumber(event: RawEvent, keys: string[]) {
  for (const key of keys) {
    const value = event[key];
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value;
    }
    if (typeof value === 'string' && value.length > 0) {
      const parsed = Number.parseInt(value, 10);
      if (Number.isFinite(parsed)) {
        return parsed;
      }
    }
  }
  return undefined;
}

function eventTopic(event: RawEvent) {
  return Array.isArray(event.topic)
    ? event.topic
    : Array.isArray(event.topics)
      ? event.topics
      : [];
}

function eventPayload(event: RawEvent) {
  return event.value ?? event.data ?? event.payload ?? event.body ?? null;
}

function normalizeEvent(resource: TrackedResource, cursorName: string, event: RawEvent) {
  const externalId =
    eventString(event, ['id']) ??
    `${resource.kind}:${resource.name}:${eventString(event, ['txHash', 'tx_hash']) ?? 'unknown'}:${eventNumber(event, ['ledger', 'ledgerSequence', 'ledger_sequence']) ?? 0}:${eventString(event, ['cursor', 'pagingToken', 'paging_token', 'pagingTokenId']) ?? 'tail'}`;
  const cursor =
    eventString(event, ['cursor', 'pagingToken', 'paging_token', 'pagingTokenId', 'id']) ?? null;
  const ledger =
    eventNumber(event, ['ledger', 'ledgerSequence', 'ledger_sequence']) ?? null;
  return {
    external_id: externalId,
    cursor_name: cursorName,
    cursor,
    resource_kind: resource.kind,
    resource_name: resource.name,
    contract_id:
      eventString(event, ['contractId', 'contract_id']) ?? resource.contractId,
    event_type: eventString(event, ['type', 'eventType', 'event_type']) ?? 'contract',
    topic: JSON.stringify(eventTopic(event)),
    payload: JSON.stringify(eventPayload(event)),
    tx_hash: eventString(event, ['txHash', 'tx_hash']) ?? null,
    ledger,
    observed_at:
      eventString(event, ['ledgerClosedAt', 'ledger_closed_at']) ??
      new Date().toISOString(),
  };
}

async function fetchEvents(
  resource: TrackedResource,
  cursor: string | null,
  config: ReturnType<typeof resolveEventWorkerConfig>,
  topicFilters: string[],
  startLedger?: number,
) {
  const args = [
    'events',
    '--output',
    'json',
    '--count',
    String(Math.min(config.batch_size, 1000)),
    '--id',
    resource.contractId,
    '--network',
    activeNetwork(),
  ];

  if (config.event_type !== 'all') {
    args.push('--type', config.event_type);
  }
  for (const topic of topicFilters) {
    args.push('--topic', topic);
  }

  if (cursor && cursor.length > 0) {
    args.push('--cursor', cursor);
  } else if (startLedger) {
    args.push('--start-ledger', String(startLedger));
  }

  const { stdout } = await execFileAsync(
    process.env.STELLAR_BIN ?? 'stellar',
    args,
    {
      cwd: projectRoot,
      maxBuffer: 16 * 1024 * 1024,
    },
  );

  return extractEvents(JSON.parse(stdout));
}

export async function ingestOnce() {
  loadForgeEnv();
  const config = resolveEventWorkerConfig();
  const topicFilters = config.topic_filters.map(normalizeTopicFilter);
  const db = openEventStore();
  try {
    const resources = trackedResources(config.resources);
    if (resources.length === 0) {
      console.warn('No deployed contracts or token wrappers found in env; skipping event ingest.');
      syncCursorSnapshot(db);
      return;
    }

    for (const resource of resources) {
      const cursorName = `${activeNetwork()}:${resource.kind}:${resource.name}`;
      const previous = loadCursor(cursorName, db);
      const events = await fetchEvents(
        resource,
        previous?.cursor ?? null,
        config,
        topicFilters,
        previous?.last_ledger ?? config.start_ledger ?? undefined,
      );

      let lastCursor = previous?.cursor ?? null;
      let lastLedger = previous?.last_ledger ?? null;

      for (const rawEvent of events) {
        const normalized = normalizeEvent(resource, cursorName, rawEvent);
        insertEvent(normalized, db);
        lastCursor = normalized.cursor ?? lastCursor;
        lastLedger = normalized.ledger ?? lastLedger;
      }

      if (events.length > 0) {
        upsertCursor(
          {
            name: cursorName,
            resource_kind: resource.kind,
            resource_name: resource.name,
            cursor: lastCursor,
            last_ledger: lastLedger,
            updated_at: new Date().toISOString(),
          },
          db,
        );
      }

      console.log(
        `[events] ${resource.kind}:${resource.name} -> ${events.length} event(s)`,
      );
    }

    syncCursorSnapshot(db);
  } finally {
    db.close();
  }
}

async function main() {
  const once = process.argv.includes('--once');
  if (once) {
    await ingestOnce();
    return;
  }

  const intervalMs = resolveEventWorkerConfig().poll_interval_ms;
  while (true) {
    await ingestOnce();
    await delay(intervalMs);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
"####
}

pub fn web_package_json() -> &'static str {
    "{\n  \"name\": \"@stellar-forge/web\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {\n    \"dev\": \"vite\",\n    \"build\": \"vite build\"\n  },\n  \"dependencies\": {\n    \"react\": \"^19.0.0\",\n    \"react-dom\": \"^19.0.0\"\n  },\n  \"devDependencies\": {\n    \"@vitejs/plugin-react\": \"^5.0.4\",\n    \"typescript\": \"^5.9.3\",\n    \"vite\": \"^7.1.11\"\n  }\n}\n"
}

pub fn web_main(_manifest: &Manifest) -> String {
    "import React from 'react';\nimport ReactDOM from 'react-dom/client';\nimport { stellarState } from './generated/stellar.js';\n\nconst styles = {\n  page: {\n    background: '#eef2f6',\n    color: '#18212f',\n    minHeight: '100vh',\n    fontFamily: 'system-ui, sans-serif',\n    padding: '24px',\n  },\n  shell: {\n    margin: '0 auto',\n    maxWidth: '1080px',\n    display: 'grid',\n    gap: '24px',\n  },\n  hero: {\n    display: 'grid',\n    gap: '8px',\n  },\n  eyebrow: {\n    fontSize: '12px',\n    fontWeight: 700,\n    textTransform: 'uppercase' as const,\n  },\n  title: {\n    fontSize: '40px',\n    lineHeight: 1.1,\n    margin: 0,\n  },\n  subtitle: {\n    margin: 0,\n    maxWidth: '60ch',\n  },\n  grid: {\n    display: 'grid',\n    gap: '24px',\n    gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',\n  },\n  section: {\n    display: 'grid',\n    gap: '12px',\n  },\n  heading: {\n    margin: 0,\n    fontSize: '18px',\n  },\n  list: {\n    listStyle: 'none',\n    margin: 0,\n    padding: 0,\n    display: 'grid',\n    gap: '12px',\n  },\n  item: {\n    border: '1px solid #c7d0da',\n    borderRadius: '8px',\n    background: '#ffffff',\n    padding: '16px',\n    display: 'grid',\n    gap: '8px',\n  },\n  row: {\n    display: 'flex',\n    gap: '8px',\n    alignItems: 'baseline',\n    flexWrap: 'wrap' as const,\n  },\n  itemTitle: {\n    fontSize: '16px',\n    fontWeight: 700,\n  },\n  badge: {\n    border: '1px solid #117a68',\n    borderRadius: '6px',\n    padding: '2px 8px',\n    fontSize: '12px',\n    color: '#0d564b',\n    background: '#dff7f1',\n  },\n  label: {\n    fontSize: '12px',\n    fontWeight: 700,\n    textTransform: 'uppercase' as const,\n  },\n  value: {\n    fontSize: '14px',\n    wordBreak: 'break-word' as const,\n  },\n  command: {\n    fontFamily: 'ui-monospace, SFMono-Regular, monospace',\n    fontSize: '13px',\n    wordBreak: 'break-word' as const,\n  },\n  empty: {\n    border: '1px dashed #c7d0da',\n    borderRadius: '8px',\n    padding: '16px',\n    background: '#f8fbfd',\n  },\n};\n\nfunction present(value: string | undefined) {\n  return value && value.length > 0 ? value : 'Pending';\n}\n\nfunction presentCursor(value: unknown) {\n  if (typeof value === 'string' && value.length > 0) {\n    return value;\n  }\n  if (value === null || value === undefined) {\n    return 'Pending';\n  }\n  return JSON.stringify(value);\n}\n\nfunction actionQueue() {\n  const commands: string[] = [];\n  const undeployedContracts = Object.entries(stellarState.contracts).filter(([name]) => {\n    return !stellarState.deployment.contracts[name]?.contract_id;\n  });\n  const pendingTokens = Object.entries(stellarState.tokens).filter(([name, token]) => {\n    const deployment = stellarState.deployment.tokens[name];\n    if (token.kind === 'asset') {\n      return !deployment?.asset || (token.with_sac && !deployment?.sac_contract_id);\n    }\n    return !deployment?.contract_id;\n  });\n\n  if (undeployedContracts.length > 0 || pendingTokens.length > 0) {\n    commands.push(`stellar forge release deploy ${stellarState.environment}`);\n  } else {\n    commands.push(`stellar forge release verify ${stellarState.environment}`);\n  }\n\n  for (const [name, contract] of Object.entries(stellarState.contracts)) {\n    if (contract.bindings.length > 0) {\n      commands.push(`stellar forge contract bind ${name} --lang ${contract.bindings.join(',')}`);\n    }\n  }\n\n  if (stellarState.events.cursor_names.length > 0) {\n    commands.push('stellar forge events cursor ls');\n  }\n\n  if (stellarState.api?.enabled) {\n    commands.push('cd apps/api && pnpm dev');\n  }\n  if (stellarState.frontend?.enabled) {\n    commands.push('cd apps/web && pnpm dev');\n  }\n\n  return commands;\n}\n\nfunction App() {\n  const contractEntries = Object.entries(stellarState.contracts);\n  const tokenEntries = Object.entries(stellarState.tokens);\n  const walletEntries = Object.entries(stellarState.wallets);\n  const cursorEntries = Object.entries(stellarState.events.cursors);\n  const commands = actionQueue();\n\n  return (\n    <main style={styles.page}>\n      <div style={styles.shell}>\n        <section style={styles.hero}>\n          <p style={styles.eyebrow}>{stellarState.environment}</p>\n          <h1 style={styles.title}>{stellarState.project.name}</h1>\n          <p style={styles.subtitle}>\n            RPC {stellarState.network?.rpc_url ?? 'not configured'}\n          </p>\n        </section>\n\n        <section style={styles.grid}>\n          <div style={styles.section}>\n            <h2 style={styles.heading}>Queue</h2>\n            <ul style={styles.list}>\n              {commands.map((command) => (\n                <li key={command} style={styles.item}>\n                  <div style={styles.label}>Command</div>\n                  <div style={styles.command}>{command}</div>\n                </li>\n              ))}\n            </ul>\n          </div>\n\n          <div style={styles.section}>\n            <h2 style={styles.heading}>Runtime</h2>\n            <ul style={styles.list}>\n              <li style={styles.item}>\n                <div style={styles.label}>Default identity</div>\n                <div style={styles.value}>{stellarState.defaults.identity}</div>\n              </li>\n              <li style={styles.item}>\n                <div style={styles.label}>Wallets</div>\n                <div style={styles.value}>{walletEntries.map(([name]) => name).join(', ') || 'None'}</div>\n              </li>\n              <li style={styles.item}>\n                <div style={styles.label}>API</div>\n                <div style={styles.value}>{stellarState.api?.enabled ? `${stellarState.api.framework} / ${stellarState.api.events_backend}` : 'Disabled'}</div>\n              </li>\n            </ul>\n          </div>\n\n          <div style={styles.section}>\n            <h2 style={styles.heading}>Events</h2>\n            <ul style={styles.list}>\n              <li style={styles.item}>\n                <div style={styles.label}>Backend</div>\n                <div style={styles.value}>{stellarState.events.backend}</div>\n              </li>\n              <li style={styles.item}>\n                <div style={styles.label}>Tracked resources</div>\n                <div style={styles.value}>\n                  {stellarState.events.contracts.length} contracts / {stellarState.events.tokens.length} tokens\n                </div>\n              </li>\n              <li style={styles.item}>\n                <div style={styles.label}>Cursors</div>\n                <div style={styles.value}>\n                  {cursorEntries.length === 0\n                    ? 'No persisted cursor'\n                    : cursorEntries.map(([name, value]) => `${name}: ${presentCursor(value)}`).join(' | ')}\n                </div>\n              </li>\n            </ul>\n          </div>\n\n          <div style={styles.section}>\n            <h2 style={styles.heading}>Contracts</h2>\n            <ul style={styles.list}>\n              {contractEntries.length === 0 ? (\n                <li style={styles.empty}>No contract declared.</li>\n              ) : (\n                contractEntries.map(([name, contract]) => {\n                  const deployment = stellarState.deployment.contracts[name];\n                  return (\n                    <li key={name} style={styles.item}>\n                      <div style={styles.row}>\n                        <span style={styles.itemTitle}>{name}</span>\n                        <span style={styles.badge}>{contract.template}</span>\n                      </div>\n                      <div>\n                        <div style={styles.label}>Alias</div>\n                        <div style={styles.value}>{contract.alias}</div>\n                      </div>\n                      <div>\n                        <div style={styles.label}>Contract ID</div>\n                        <div style={styles.value}>{present(deployment?.contract_id)}</div>\n                      </div>\n                    </li>\n                  );\n                })\n              )}\n            </ul>\n          </div>\n\n          <div style={styles.section}>\n            <h2 style={styles.heading}>Tokens</h2>\n            <ul style={styles.list}>\n              {tokenEntries.length === 0 ? (\n                <li style={styles.empty}>No token declared.</li>\n              ) : (\n                tokenEntries.map(([name, token]) => {\n                  const deployment = stellarState.deployment.tokens[name];\n                  return (\n                    <li key={name} style={styles.item}>\n                      <div style={styles.row}>\n                        <span style={styles.itemTitle}>{name}</span>\n                        <span style={styles.badge}>{token.kind}</span>\n                      </div>\n                      <div>\n                        <div style={styles.label}>Code</div>\n                        <div style={styles.value}>{token.code || 'XLM'}</div>\n                      </div>\n                      <div>\n                        <div style={styles.label}>Asset</div>\n                        <div style={styles.value}>{present(deployment?.asset)}</div>\n                      </div>\n                      <div>\n                        <div style={styles.label}>SAC</div>\n                        <div style={styles.value}>{present(deployment?.sac_contract_id)}</div>\n                      </div>\n                    </li>\n                  );\n                })\n              )}\n            </ul>\n          </div>\n        </section>\n      </div>\n    </main>\n  );\n}\n\nReactDOM.createRoot(document.getElementById('root')!).render(<App />);\n".to_string()
}

pub fn web_generated_state(
    manifest: &Manifest,
    lockfile: &Lockfile,
    event_cursors: &Value,
    env: &str,
) -> String {
    let deployment = lockfile.environments.get(env).cloned().unwrap_or_default();
    let network = manifest.networks.get(env).cloned();
    let event_cursor_map = event_cursors
        .get("cursors")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let event_contracts = manifest.contracts.keys().cloned().collect::<Vec<_>>();
    let event_tokens = manifest.tokens.keys().cloned().collect::<Vec<_>>();
    let cursor_names = event_cursor_map
        .as_object()
        .map(|cursors| cursors.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    format!(
        "export const stellarState = {} as const;\n",
        serde_json::to_string_pretty(&json!({
            "project": manifest.project,
            "environment": env,
            "defaults": manifest.defaults,
            "network": network,
            "api": manifest.api,
            "frontend": manifest.frontend,
            "wallets": manifest.wallets,
            "contracts": manifest.contracts,
            "tokens": manifest.tokens,
            "events": {
                "backend": manifest.api.as_ref().map(|api| api.events_backend.clone()).unwrap_or_else(|| "rpc-poller".to_string()),
                "contracts": event_contracts,
                "tokens": event_tokens,
                "cursor_names": cursor_names,
                "cursors": event_cursor_map,
            },
            "deployment": deployment,
        }))
        .expect("frontend generated state should serialize")
    )
}

pub fn web_index_html() -> &'static str {
    "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"UTF-8\" />\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n    <title>stellar forge</title>\n  </head>\n  <body>\n    <div id=\"root\"></div>\n    <script type=\"module\" src=\"/src/main.tsx\"></script>\n  </body>\n</html>\n"
}

pub fn worker_stub() -> &'static str {
    r####"#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

function loadDefaults() {
  const defaults = {};
  for (const relative of ['.env.generated', '.env.example', 'apps/api/.env', 'apps/api/.env.example']) {
    const file = resolve(root, relative);
    if (!existsSync(file)) {
      continue;
    }
    for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      const separator = trimmed.indexOf('=');
      if (separator === -1) {
        continue;
      }
      const key = trimmed.slice(0, separator).trim();
      const value = trimmed.slice(separator + 1).trim();
      if (value && !(key in defaults)) {
        defaults[key] = value;
      }
    }
  }
  return defaults;
}

function runForge(args) {
  const command = process.env.STELLAR_FORGE_BIN || 'stellar-forge';
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
  if (result.error) {
    if (result.error.code === 'ENOENT') {
      console.error(`Could not find ${command} on PATH.`);
      console.error('Set STELLAR_FORGE_BIN to override the executable name.');
      process.exit(1);
    }
    throw result.error;
  }
  return result.status ?? 1;
}

function sleep(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

const defaults = loadDefaults();
const rawArgs = process.argv.slice(2);
const once = rawArgs.includes('--once');
const filteredArgs = rawArgs.filter((arg) => arg !== '--once');
const explicitResource = filteredArgs.find((arg) => !arg.startsWith('-'));
const configuredResource = (defaults.STELLAR_EVENTS_RESOURCES || '')
  .split(',')
  .map((value) => value.trim())
  .find((value) => value.length > 0);
const resource = explicitResource || configuredResource;

if (!resource) {
  console.error('Provide a resource like `rewards`, `token:points`, or `account:alice`.');
  console.error('You can also set STELLAR_EVENTS_RESOURCES in apps/api/.env.example.');
  process.exit(1);
}

const network = defaults.STELLAR_NETWORK || 'testnet';
const count = defaults.STELLAR_EVENTS_BATCH_SIZE || '200';
const intervalMs = Number(defaults.STELLAR_EVENTS_POLL_INTERVAL_MS || '5000');
const hasNetworkFlag = filteredArgs.some((arg) => arg === '--network' || arg.startsWith('--network='));
const hasCountFlag = filteredArgs.some((arg) => arg === '--count' || arg.startsWith('--count='));
const forwarded = filteredArgs.filter((arg) => arg !== resource);

async function ingestLoop() {
  do {
    const args = [
      ...(hasNetworkFlag ? [] : ['--network', network]),
      'events',
      'backfill',
      resource,
      ...(hasCountFlag ? [] : ['--count', count]),
      ...forwarded,
    ];
    const status = runForge(args);
    if (status !== 0) {
      process.exit(status);
    }
    if (once) {
      return;
    }
    await sleep(intervalMs);
  } while (true);
}

await ingestLoop();
"####
}

pub fn contract_rust_toolchain() -> &'static str {
    "[toolchain]\nchannel = \"stable\"\nprofile = \"minimal\"\ntargets = [\"wasm32v1-none\"]\n"
}

pub fn smart_wallet_readme(
    name: &str,
    mode: &str,
    policy_contract: &str,
    controller_identity: Option<&str>,
) -> String {
    let intro = if mode == "ed25519" {
        "This scaffold prepares a controller-backed smart wallet flow.\nUse it as the browser or operator entrypoint for policy deployment and contract-account provisioning.\n"
    } else {
        "This scaffold is meant for passkey or policy-driven onboarding flows.\nUse it as the browser-facing entrypoint for WebAuthn and contract-account setup.\n"
    };
    let mut steps =
        vec!["1. Configure `.env.example` with the policy contract and RPC URL.".to_string()];
    if let Some(controller_identity) = controller_identity {
        steps.push(format!(
            "{}. Generate or verify the controller identity `{controller_identity}` and fund it on your target network.",
            steps.len() + 1
        ));
    }
    steps.push(format!(
        "{}. Build and deploy the generated `{policy_contract}` contract.",
        steps.len() + 1
    ));
    steps.push(if mode == "ed25519" {
        format!(
            "{}. Replace the stub with your controller-signing and contract-account provisioning flow.",
            steps.len() + 1
        )
    } else {
        format!(
            "{}. Start the onboarding app with your preferred package manager and connect the browser flow.",
            steps.len() + 1
        )
    });
    format!(
        "# Smart wallet scaffold: {name}\n\n{intro}\n## Suggested flow\n\n{}\n",
        steps.join("\n")
    )
}

pub fn smart_wallet_env_example(
    name: &str,
    mode: &str,
    policy_contract: &str,
    controller_identity: Option<&str>,
) -> String {
    let mut lines = vec![
        format!("SMART_WALLET_NAME={name}"),
        format!("SMART_WALLET_MODE={mode}"),
        format!("SMART_WALLET_POLICY_CONTRACT={policy_contract}"),
    ];
    if let Some(controller_identity) = controller_identity {
        lines.push(format!(
            "SMART_WALLET_CONTROLLER_IDENTITY={controller_identity}"
        ));
    }
    lines.push("SMART_WALLET_CONTRACT_ID=".to_string());
    lines.push("SMART_WALLET_RPC_URL=https://soroban-testnet.stellar.org".to_string());
    format!("{}\n", lines.join("\n"))
}

pub fn smart_wallet_package_json(name: &str) -> String {
    format!(
        "{{\n  \"name\": \"{name}-smart-wallet\",\n  \"private\": true,\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"dev\": \"vite\",\n    \"build\": \"tsc && vite build\",\n    \"preview\": \"vite preview\"\n  }},\n  \"devDependencies\": {{\n    \"typescript\": \"^5.9.0\",\n    \"vite\": \"^7.1.0\"\n  }}\n}}\n"
    )
}

pub fn smart_wallet_tsconfig() -> &'static str {
    "{\n  \"compilerOptions\": {\n    \"target\": \"ES2020\",\n    \"module\": \"ESNext\",\n    \"moduleResolution\": \"Bundler\",\n    \"strict\": true,\n    \"skipLibCheck\": true,\n    \"types\": [\"vite/client\"]\n  },\n  \"include\": [\"src/**/*.ts\"]\n}\n"
}

pub fn smart_wallet_index_html() -> &'static str {
    "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"UTF-8\" />\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n    <title>Smart Wallet Onboarding</title>\n  </head>\n  <body>\n    <div id=\"app\"></div>\n    <script type=\"module\" src=\"/src/main.ts\"></script>\n  </body>\n</html>\n"
}

pub fn smart_wallet_main_ts(
    name: &str,
    mode: &str,
    policy_contract: &str,
    controller_identity: Option<&str>,
) -> String {
    let subtitle = if mode == "ed25519" {
        "Controller-backed onboarding scaffold for a Stellar smart wallet."
    } else {
        "Passkey onboarding scaffold for a Stellar smart wallet."
    };
    let controller_item = controller_identity
        .map(|controller_identity| {
            format!(
                "<li>Use the controller identity <code>{controller_identity}</code> to sign the provisioning transaction.</li>\n        "
            )
        })
        .unwrap_or_default();
    let controller_env = controller_identity
        .map(|controller_identity| {
            format!("\\nSMART_WALLET_CONTROLLER_IDENTITY={controller_identity}")
        })
        .unwrap_or_default();
    let last_item = if mode == "ed25519" {
        "Replace this stub with your controller-signing and contract-account deployment flow."
    } else {
        "Replace this stub with your WebAuthn ceremony and contract-account provisioning flow."
    };
    format!(
        "const app = document.querySelector<HTMLDivElement>('#app');\n\nif (!app) {{\n  throw new Error('missing app root');\n}}\n\napp.innerHTML = `\n  <main style=\"font-family: Inter, system-ui, sans-serif; max-width: 760px; margin: 0 auto; padding: 40px 24px; line-height: 1.5;\">\n    <h1 style=\"margin-bottom: 8px;\">{name}</h1>\n    <p style=\"margin-top: 0; color: #475569;\">{subtitle}</p>\n    <section style=\"margin-top: 24px;\">\n      <h2 style=\"margin-bottom: 8px;\">Checklist</h2>\n      <ol>\n        <li>Deploy the <code>{policy_contract}</code> policy contract.</li>\n        <li>Set <code>SMART_WALLET_CONTRACT_ID</code> and <code>SMART_WALLET_RPC_URL</code>.</li>\n        {controller_item}<li>{last_item}</li>\n      </ol>\n    </section>\n    <section style=\"margin-top: 24px;\">\n      <h2 style=\"margin-bottom: 8px;\">Environment</h2>\n      <pre style=\"padding: 16px; background: #0f172a; color: #e2e8f0; overflow: auto; border-radius: 8px;\">SMART_WALLET_NAME={name}\nSMART_WALLET_MODE={mode}\nSMART_WALLET_POLICY_CONTRACT={policy_contract}{controller_env}\nSMART_WALLET_CONTRACT_ID=\nSMART_WALLET_RPC_URL=https://soroban-testnet.stellar.org</pre>\n    </section>\n  </main>\n`;\n"
    )
}

pub fn contract_template_files(template: &str, name: &str) -> Option<ContractTemplateFiles> {
    match template {
        "rewards" => Some(ContractTemplateFiles {
            lib_rs: rewards_contract_lib(),
            test_rs: Some(rewards_contract_test()),
            readme: format!(
                "# {name}\n\nTemplate tuned for loyalty and rewards flows.\n\nThe generated contract stores an admin address, a token address, and per-user points balances. It is a small starting point for demos and hackathon flows.\n"
            ),
        }),
        "openzeppelin-token" => Some(ContractTemplateFiles {
            lib_rs: standard_token_contract_lib(),
            test_rs: Some(standard_token_contract_test()),
            readme: format!(
                "# {name}\n\nTemplate tuned for contract-token flows.\n\nThe generated contract exposes `init`, `mint`, `burn`, `transfer`, and `balance` so the CLI can orchestrate contract-token lifecycle commands with a local, workspace-owned implementation.\n"
            ),
        }),
        "passkey-wallet-policy" => Some(ContractTemplateFiles {
            lib_rs: passkey_wallet_policy_contract_lib(),
            test_rs: Some(passkey_wallet_policy_contract_test()),
            readme: format!(
                "# {name}\n\nTemplate tuned for smart-wallet policy flows.\n\nThe generated contract stores an admin, a daily spend limit, and a small allow list so the onboarding app has a local policy contract to evolve from.\n"
            ),
        }),
        "escrow" => Some(ContractTemplateFiles {
            lib_rs: escrow_contract_lib(),
            test_rs: Some(escrow_contract_test()),
            readme: format!(
                "# {name}\n\nTemplate tuned for escrow-like release flows.\n\nThe generated contract tracks payer, payee, amount, and release state so the project can start from domain language that matches the scaffold name.\n"
            ),
        }),
        _ => None,
    }
}

fn sample_contract(name: &str, template: &str) -> ContractConfig {
    ContractConfig {
        path: format!("contracts/{name}"),
        alias: name.to_string(),
        template: template.to_string(),
        bindings: vec!["typescript".to_string()],
        deploy_on: vec!["local".to_string(), "testnet".to_string()],
        init: None,
    }
}

fn add_issuer_wallet(
    identities: &mut BTreeMap<String, IdentityConfig>,
    wallets: &mut BTreeMap<String, WalletConfig>,
    tokens: &mut BTreeMap<String, TokenConfig>,
) {
    for name in ["issuer", "treasury"] {
        identities.insert(
            name.to_string(),
            IdentityConfig {
                source: "stellar-cli".to_string(),
                name: name.to_string(),
            },
        );
        wallets.insert(
            name.to_string(),
            WalletConfig {
                kind: "classic".to_string(),
                identity: name.to_string(),
                controller_identity: None,
                mode: None,
                onboarding_app: None,
                policy_contract: None,
            },
        );
    }
    tokens.insert(
        "points".to_string(),
        TokenConfig {
            kind: "asset".to_string(),
            code: "POINTS".to_string(),
            issuer: "@identity:issuer".to_string(),
            distribution: "@identity:treasury".to_string(),
            auth_required: true,
            auth_revocable: true,
            clawback_enabled: true,
            with_sac: true,
            decimals: 7,
            metadata_name: Some("Loyalty Points".to_string()),
        },
    );
}

fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|char| match char {
            'a'..='z' | '0'..='9' => char,
            'A'..='Z' => char.to_ascii_lowercase(),
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn rewards_contract_lib() -> String {
    r#"#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Points(Address),
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn init(env: Env, admin: Address, token: Address) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "contract already initialized"
        );
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn award_points(env: Env, player: Address, amount: i128) -> i128 {
        assert!(amount > 0, "amount must be positive");
        let next = Self::points(env.clone(), player.clone()) + amount;
        env.storage().persistent().set(&DataKey::Points(player), &next);
        next
    }

    pub fn spend_points(env: Env, player: Address, amount: i128) -> i128 {
        assert!(amount > 0, "amount must be positive");
        let current = Self::points(env.clone(), player.clone());
        assert!(current >= amount, "insufficient points");
        let next = current - amount;
        env.storage().persistent().set(&DataKey::Points(player), &next);
        next
    }

    pub fn points(env: Env, player: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Points(player))
            .unwrap_or(0)
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn token(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Token).unwrap()
    }
}

mod test;
"#
    .to_string()
}

fn rewards_contract_test() -> String {
    r#"#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn rewards_flow_tracks_points() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let player = Address::generate(&env);

    client.init(&admin, &token);
    assert_eq!(client.admin(), admin);
    assert_eq!(client.token(), token);
    assert_eq!(client.points(&player), 0);
    assert_eq!(client.award_points(&player, &120), 120);
    assert_eq!(client.spend_points(&player, &20), 100);
    assert_eq!(client.points(&player), 100);
}
"#
    .to_string()
}

fn standard_token_contract_lib() -> String {
    r#"#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Name,
    Symbol,
    Decimals,
    Balance(Address),
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn init(env: Env, admin: Address, name: String, symbol: String, decimals: u32) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "contract already initialized"
        );
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
    }

    pub fn mint(env: Env, to: Address, amount: i128) -> i128 {
        let admin = Self::admin(env.clone());
        admin.require_auth();
        assert!(amount > 0, "amount must be positive");
        let next = Self::balance(env.clone(), to.clone()) + amount;
        env.storage().persistent().set(&DataKey::Balance(to), &next);
        next
    }

    pub fn burn(env: Env, from: Address, amount: i128) -> i128 {
        from.require_auth();
        assert!(amount > 0, "amount must be positive");
        let current = Self::balance(env.clone(), from.clone());
        assert!(current >= amount, "insufficient balance");
        let next = current - amount;
        env.storage().persistent().set(&DataKey::Balance(from), &next);
        next
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> bool {
        from.require_auth();
        assert!(amount > 0, "amount must be positive");
        let from_balance = Self::balance(env.clone(), from.clone());
        assert!(from_balance >= amount, "insufficient balance");
        let to_balance = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(from_balance - amount));
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_balance + amount));
        true
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn name(env: Env) -> String {
        env.storage().instance().get(&DataKey::Name).unwrap()
    }

    pub fn symbol(env: Env) -> String {
        env.storage().instance().get(&DataKey::Symbol).unwrap()
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap()
    }
}

mod test;
"#
    .to_string()
}

fn standard_token_contract_test() -> String {
    r#"#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn contract_token_flow_supports_init_mint_transfer_and_burn() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    client.init(
        &admin,
        &String::from_str(&env, "Store Credit"),
        &String::from_str(&env, "CREDIT"),
        &7u32,
    );
    assert_eq!(client.name(), String::from_str(&env, "Store Credit"));
    assert_eq!(client.symbol(), String::from_str(&env, "CREDIT"));
    assert_eq!(client.decimals(), 7);
    assert_eq!(client.mint(&alice, &500), 500);
    assert!(client.transfer(&alice, &bob, &120));
    assert_eq!(client.balance(&alice), 380);
    assert_eq!(client.burn(&bob, &20), 100);
}
"#
    .to_string()
}

fn passkey_wallet_policy_contract_lib() -> String {
    r#"#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    DailyLimit,
    Allowed(Address),
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn init(env: Env, admin: Address, daily_limit: i128) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "contract already initialized"
        );
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::DailyLimit, &daily_limit);
    }

    pub fn set_daily_limit(env: Env, daily_limit: i128) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::DailyLimit, &daily_limit);
    }

    pub fn allow(env: Env, address: Address) {
        Self::require_admin(&env);
        env.storage().persistent().set(&DataKey::Allowed(address), &true);
    }

    pub fn revoke(env: Env, address: Address) {
        Self::require_admin(&env);
        env.storage().persistent().set(&DataKey::Allowed(address), &false);
    }

    pub fn is_allowed(env: Env, address: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Allowed(address))
            .unwrap_or(false)
    }

    pub fn daily_limit(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::DailyLimit).unwrap_or(0)
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
    }
}

mod test;
"#
    .to_string()
}

fn passkey_wallet_policy_contract_test() -> String {
    r#"#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn policy_template_tracks_admin_limit_and_allow_list() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let spender = Address::generate(&env);

    client.init(&admin, &500);
    assert_eq!(client.admin(), admin);
    assert_eq!(client.daily_limit(), 500);
    assert!(!client.is_allowed(&spender));

    client.allow(&spender);
    assert!(client.is_allowed(&spender));

    client.set_daily_limit(&1250);
    assert_eq!(client.daily_limit(), 1250);

    client.revoke(&spender);
    assert!(!client.is_allowed(&spender));
}
"#
    .to_string()
}

fn escrow_contract_lib() -> String {
    r#"#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Payer,
    Payee,
    Amount,
    Released,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn init(env: Env, payer: Address, payee: Address, amount: i128) {
        assert!(
            !env.storage().instance().has(&DataKey::Payer),
            "contract already initialized"
        );
        assert!(amount > 0, "amount must be positive");
        env.storage().instance().set(&DataKey::Payer, &payer);
        env.storage().instance().set(&DataKey::Payee, &payee);
        env.storage().instance().set(&DataKey::Amount, &amount);
        env.storage().instance().set(&DataKey::Released, &false);
    }

    pub fn release(env: Env) -> bool {
        env.storage().instance().set(&DataKey::Released, &true);
        true
    }

    pub fn payer(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Payer).unwrap()
    }

    pub fn payee(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Payee).unwrap()
    }

    pub fn amount(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::Amount).unwrap()
    }

    pub fn is_released(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Released).unwrap_or(false)
    }
}

mod test;
"#
    .to_string()
}

fn escrow_contract_test() -> String {
    r#"#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn escrow_flow_tracks_release() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let payer = Address::generate(&env);
    let payee = Address::generate(&env);

    client.init(&payer, &payee, &500);
    assert_eq!(client.payer(), payer);
    assert_eq!(client.payee(), payee);
    assert_eq!(client.amount(), 500);
    assert!(!client.is_released());
    assert!(client.release());
    assert!(client.is_released());
}
"#
    .to_string()
}
