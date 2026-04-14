# Command Reference

Examples use `stellar forge ...`. Replace them with `stellar-forge ...` if the plugin is not
visible through `stellar plugin ls`.

## Global Flags

These flags apply to almost every command:

| Flag | Meaning |
| --- | --- |
| `--manifest <path>` | Read a manifest from a different path |
| `--cwd <path>` | Change the working directory the CLI uses |
| `--network <name>` | Override `defaults.network` |
| `--identity <name>` | Override `defaults.identity` |
| `--json` | Emit structured JSON reports |
| `--quiet` | Reduce non-essential output |
| `--verbose` / `-vv` | Increase verbosity |
| `--dry-run` | Preview the commands and artifacts without making changes |
| `--yes` | Skip confirmations in flows that support them |

Recommended habits:

```bash
stellar forge --dry-run project validate
stellar forge --dry-run release plan testnet
stellar forge --json doctor
```

## `init`

Bootstrap a new managed workspace.

### Syntax

```bash
stellar forge init <name> [--template <template>] [--network <network>] [--contracts <n>]
```

### Common flags

| Flag | Meaning |
| --- | --- |
| `--template <template>` | One of `minimal-contract`, `fullstack`, `issuer-wallet`, `merchant-checkout`, `rewards-loyalty`, `api-only`, `multi-contract` |
| `--frontend <framework>` | Frontend scaffold name, default `react-vite` |
| `--api` / `--no-api` | API generation toggle; defaults to enabled |
| `--package-manager <pm>` | Default `pnpm` |
| `--contracts <n>` | Number of contract entries to scaffold |
| `--install` | Run the package manager install step in generated apps |
| `--git` | Run `git init` inside the new project |
| `--no-api` | Skip API generation even when the template normally includes it |

### Template intent

| Template | What it creates |
| --- | --- |
| `minimal-contract` | Starts from a single contract; API still follows `--api` / `--no-api` |
| `fullstack` | Contract + API + frontend |
| `issuer-wallet` | Issuer/treasury wallets, a sample points token, API, and frontend |
| `merchant-checkout` | Checkout-oriented starter with wallet/token scaffolding |
| `rewards-loyalty` | Loyalty token, rewards contract, API, frontend, and release defaults |
| `api-only` | API scaffold without contract/frontend defaults |
| `multi-contract` | Starts from multiple contracts; API still follows `--api` / `--no-api` |

### Examples

```bash
stellar forge init hello-stellar --template fullstack
stellar forge init rewards-app --template rewards-loyalty --network testnet
stellar forge init minimal --template minimal-contract --contracts 1 --no-api
```

## `project`

Inspect, validate, synchronize, extend, or adopt a workspace.

### `project info`

Prints the manifest, release config, deployment state, and Scaffold-compatibility snapshot.

```bash
stellar forge project info
stellar forge --json project info
```

### `project sync`

Regenerates derived files from the manifest, including `.env.example`, API files, frontend state,
and OpenAPI output when those modules are enabled.

```bash
stellar forge project sync
```

### `project validate`

Strict validation wrapper over project diagnostics. In human mode it exits non-zero when errors are
found.

```bash
stellar forge project validate
stellar forge --json project validate
```

### `project add`

Adds a managed module to an existing workspace.

```bash
stellar forge project add contract escrow --template escrow
stellar forge project add api
stellar forge project add frontend --framework react-vite
```

### `project adopt scaffold`

Imports an existing Scaffold-style workspace into `stellarforge.toml` and
`stellarforge.lock.json`.

What it attempts to import:

- contracts under `contracts/`
- generated bindings under `packages/`
- environment definitions from `environments.toml`
- deploy aliases and existing contract IDs
- a root frontend if present

```bash
stellar forge project adopt scaffold
```

## `doctor`

Runs environment, dependency, project, and network diagnostics.

### Common forms

```bash
stellar forge doctor
stellar forge doctor env
stellar forge doctor deps
stellar forge doctor project
stellar forge doctor network testnet
```

### What each subcommand checks

| Command | Focus |
| --- | --- |
| `doctor` | Full diagnostic sweep |
| `doctor env` | Active cwd, manifest path, network, identity, output mode |
| `doctor deps` | Presence of `stellar`, Docker, Rust, Node, `pnpm`, `sqlite3`, registry tooling, plugin detection |
| `doctor project` | Manifest validity, generated files, lockfile, release-state drift, scaffold compatibility |
| `doctor network <env>` | RPC/Horizon reachability and deployed-resource probes for a target environment |

## `dev`

Controls local quickstart flows and reseeding.

### `dev up`

Starts the local Stellar quickstart through `stellar container start local` and writes a local
`.env.generated`.

```bash
stellar forge dev up
```

### `dev down`

Stops the local quickstart.

```bash
stellar forge dev down
```

### `dev status`

Runs the same kind of checks as `doctor network local`.

```bash
stellar forge dev status
```

### `dev reset`

Restarts the local quickstart.

```bash
stellar forge dev reset
```

### `dev reseed`

Rehydrates identities, tokens, contracts, event state, and env exports for the selected network.

```bash
stellar forge dev reseed
stellar forge dev reseed --network local
stellar forge dev reseed --network testnet
```

### `dev fund <target>`

Funds a named wallet, identity, or direct address using friendbot or the local root account.

```bash
stellar forge dev fund alice
stellar forge dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
```

### `dev watch`

Polls contract source trees, rebuilds changed contracts, and refreshes generated API/frontend
files.

```bash
stellar forge dev watch
stellar forge dev watch --once
stellar forge dev watch --interval-ms 3000
```

Notes:

- `--json` requires `--once`
- this is file watching for managed contracts, not on-chain event streaming

### `dev logs`

Streams local quickstart logs.

```bash
stellar forge dev logs
```

## `contract`

Build, deploy, inspect, fetch, bind, invoke, and manage TTL for contracts.

### `contract new`

```bash
stellar forge contract new hello --template basic
stellar forge contract new escrow --template escrow
```

### `contract build`

Build one contract or all declared contracts.

```bash
stellar forge contract build
stellar forge contract build rewards
stellar forge contract build rewards --optimize
```

### `contract deploy`

Deploys a single declared contract and updates the lockfile.

```bash
stellar forge contract deploy rewards
stellar forge contract deploy rewards --env testnet
```

### `contract call`

Syntax:

```bash
stellar forge contract call <contract> <function> [--send <mode>] [--build-only] [-- <args...>]
```

Important detail: arguments after the function are passed through to `stellar contract invoke`, so
the `--` separator matters.

Examples:

```bash
stellar forge contract call rewards award_points -- --member alice --amount 100
stellar forge contract call rewards spend_points --send no -- --member alice --amount 50
stellar forge contract call rewards award_points --build-only -- --member alice --amount 25
```

### `contract bind`

Generates bindings under `packages/`. If `--lang` is omitted, the implementation defaults to
TypeScript.

```bash
stellar forge contract bind rewards --lang typescript,python
```

### `contract info`

Summarizes manifest, deployment, bindings, and contract metadata.

```bash
stellar forge contract info rewards
```

### `contract fetch`

Fetches Wasm from the network into `dist/contracts/<name>.<env>.wasm` unless `--out` is supplied.

```bash
stellar forge contract fetch rewards
stellar forge contract fetch rewards --out ./tmp/rewards.wasm
```

### `contract spec`

Prints contract info oriented around interface/spec inspection.

```bash
stellar forge contract spec rewards
```

### `contract ttl extend|restore`

Manage TTL with the underlying Stellar CLI.

```bash
stellar forge contract ttl extend rewards --ledgers 17280
stellar forge contract ttl restore rewards --key all
stellar forge contract ttl extend rewards --durability temporary --build-only
```

## `token`

Create and operate asset tokens, SAC wrappers, and contract-token projects.

### `token create`

Syntax:

```bash
stellar forge token create <name> [--mode asset|contract] [--with-sac]
```

Asset token example:

```bash
stellar forge token create points \
  --mode asset \
  --issuer issuer \
  --distribution treasury \
  --with-sac \
  --initial-supply 1000000 \
  --auth-required \
  --auth-revocable \
  --clawback-enabled \
  --metadata-name "Loyalty Points"
```

Contract token example:

```bash
stellar forge token create credits \
  --mode contract \
  --issuer issuer \
  --distribution treasury \
  --initial-supply 500000 \
  --metadata-name "Credits"
```

Notes:

- `--mode contract` scaffolds a matching contract, deploys it, initializes it, generates bindings,
  and optionally mints the initial supply
- `--with-sac` is meaningful for classic asset tokens and enables a SAC deploy path

### `token info`

```bash
stellar forge token info points
```

### `token mint`

```bash
stellar forge token mint points --to alice --amount 100
stellar forge token mint credits --to alice --amount 100 --from issuer
```

### `token burn`

```bash
stellar forge token burn points --from alice --amount 10
stellar forge token burn credits --from alice --amount 10
```

### `token transfer`

```bash
stellar forge token transfer points --from alice --to bob --amount 25
stellar forge token transfer credits --from alice --to bob --amount 25
```

### `token trust`

Alias for wallet trustline creation. Only valid for classic asset tokens.

```bash
stellar forge token trust points alice
```

### `token freeze`, `token unfreeze`, `token clawback`

Classic-asset admin flows:

```bash
stellar forge token freeze points alice
stellar forge token unfreeze points alice
stellar forge token clawback points alice 25
```

### `token sac id`, `token sac deploy`

Work with Stellar Asset Contract wrappers for asset tokens.

```bash
stellar forge token sac id points
stellar forge token sac deploy points
```

### `token contract init`

Runs the contract-token initialization flow declared in the manifest.

```bash
stellar forge token contract init credits
```

### `token balance`

```bash
stellar forge token balance points --holder alice
stellar forge token balance credits --holder bob
```

## `wallet`

Manage classic wallets, build payments, generate SEP-7 payloads, and scaffold smart-wallet helpers.

### `wallet create`

```bash
stellar forge wallet create alice
stellar forge wallet create alice --fund
```

### `wallet ls`

Lists Stellar identities and declared wallets.

```bash
stellar forge wallet ls
```

### `wallet address`

```bash
stellar forge wallet address alice
```

### `wallet fund`

```bash
stellar forge wallet fund alice
stellar forge wallet fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
```

### `wallet balances`

Shows Horizon balances plus project-token lookups, including SAC and contract-token balances when
possible.

```bash
stellar forge wallet balances alice
```

### `wallet trust`

Creates a classic trustline for an asset token.

```bash
stellar forge wallet trust alice points
```

### `wallet pay`

Syntax:

```bash
stellar forge wallet pay --from <from> --to <to> --asset <asset> --amount <amount>
```

Examples:

```bash
stellar forge wallet pay --from alice --to bob --asset XLM --amount 10
stellar forge wallet pay --from alice --to bob --asset points --amount 25
stellar forge wallet pay --from alice --to bob --asset points --amount 25 --sep7
stellar forge wallet pay --from alice --to bob --asset points --amount 25 --relayer
stellar forge wallet pay --from alice --to bob --asset points --amount 25 --build-only
```

The command chooses the right primitive for the asset you pass: native payment, classic asset
payment, SAC flow, or contract-token flow.

### `wallet receive`

Print a wallet address, and optionally return SEP-7 and QR payload data.

```bash
stellar forge wallet receive alice
stellar forge wallet receive alice --sep7 --asset points
stellar forge wallet receive alice --qr --asset XLM
```

Notes:

- SEP-7 payment URIs work for classic assets
- contract tokens are not directly representable as SEP-7 payment URIs, so the command falls back
  to the raw address with a warning

### `wallet sep7 payment`

Explicit SEP-7 form of `wallet pay`.

```bash
stellar forge wallet sep7 payment --from alice --to bob --asset points --amount 25
```

### `wallet sep7 contract-call`

Build a SEP-7 payload for a contract call.

```bash
stellar forge wallet sep7 contract-call rewards spend_points -- --member alice --amount 5
```

### `wallet smart`

Scaffolds smart-wallet-adjacent helpers and metadata.

```bash
stellar forge wallet smart create team-safe --mode ed25519
stellar forge wallet smart create checkout-passkey --mode passkey
stellar forge wallet smart scaffold checkout-passkey
stellar forge wallet smart info checkout-passkey
```

## `api`

Generate or refresh the managed API scaffold.

### `api init`

Creates `apps/api` and turns API support on in the manifest.

```bash
stellar forge api init
```

### `api generate contract|token`

Generates a resource service around a specific contract or token.

```bash
stellar forge api generate contract rewards
stellar forge api generate token points
```

### `api openapi export`

Regenerates `apps/api/openapi.json`.

```bash
stellar forge api openapi export
```

### `api events init`

Ensures event-ingestion support exists in the API scaffold.

```bash
stellar forge api events init
```

### `api relayer init`

Adds relayer endpoints and support files to the API scaffold.

```bash
stellar forge api relayer init
```

## `events`

Stream contract/account events, seed recent event history, and manage cursors.

### `events watch`

Syntax:

```bash
stellar forge events watch <kind> <resource>
```

Supported kinds are `contract`, `token`, and `account`.

Examples:

```bash
stellar forge events watch contract rewards
stellar forge events watch token points
stellar forge events watch account alice
stellar forge events watch contract rewards --topic sym:PointsAwarded
stellar forge events watch contract rewards --count 100 --start-ledger 123456
```

### `events ingest init`

Bootstraps event-ingestion support. Internally it aligns with API event scaffold generation.

```bash
stellar forge events ingest init
```

### `events cursor ls`

Lists persisted event cursors from sqlite or the JSON snapshot.

```bash
stellar forge events cursor ls
```

### `events cursor reset`

```bash
stellar forge events cursor reset testnet:contract:rewards
```

### `events backfill`

Backfills recent history into the local sqlite event store. The command is retention-bound by the
provider behind the RPC/Horizon endpoints.

Accepted resource forms:

- contract name, for example `rewards`
- token name, for example `points`
- explicit prefix, for example `contract:rewards`, `token:points`, `account:alice`

Examples:

```bash
stellar forge events backfill contract:rewards --count 200
stellar forge events backfill points --start-ledger 123456
stellar forge events backfill account:alice --count 100
```

Notes:

- this command requires the API scaffold and `sqlite3`
- public RPC history is short-lived; use backfill to seed recent history, not as a permanent archive
- topic filters support comma-separated segments and wildcards such as `COUNTER,*` or `sym:Transfer,*`

## `release`

Plan, deploy, verify, export env, sync aliases, and run registry workflows.

### `release plan <env>`

Shows the commands that would run, the identities required, expected lockfile changes, and which
artifacts will be produced.

```bash
stellar forge release plan testnet
stellar forge --dry-run release plan futurenet
```

### `release deploy <env>`

Performs the release for a target environment.

```bash
stellar forge release deploy testnet
stellar forge release deploy pubnet --confirm-mainnet
```

Important behavior:

- if `[release.<env>]` exists, only its listed contracts and tokens are deployed
- otherwise all declared contracts and tokens are considered part of the release
- `pubnet` is guarded by `--confirm-mainnet`

### `release verify <env>`

Validates deploy artifacts, lockfile consistency, event worker config, and optionally probes
deployed contract IDs on-chain.

```bash
stellar forge release verify testnet
```

### `release aliases sync <env>`

Synchronizes Stellar CLI aliases from manifest and lockfile state.

```bash
stellar forge release aliases sync testnet
```

### `release env export <env>`

Writes `.env.generated` and the deploy snapshot for the target environment.

```bash
stellar forge release env export testnet
```

### `release registry publish <contract>`

Publishes registry metadata for a contract using the active network from `--network` or
`defaults.network`.

```bash
stellar forge --network testnet release registry publish rewards
```

### `release registry deploy <contract>`

Deploys a contract using registry metadata and updates `dist/registry.<env>.json`.

```bash
stellar forge --network testnet release registry deploy rewards
```

Registry notes:

- the command first tries `stellar registry ...`
- if that subcommand is unavailable, it can fall back to the standalone `stellar-registry` binary
- set `STELLAR_FORGE_REGISTRY_MODE=stellar` or `STELLAR_FORGE_REGISTRY_MODE=dedicated` for a
  deterministic backend choice

## Generated Helper Scripts

Fresh projects include small Node wrappers under `scripts/`:

| Script | What it does |
| --- | --- |
| `node scripts/doctor.mjs` | Runs `stellar-forge doctor` using `.env.generated` / `.env.example` defaults |
| `node scripts/reseed.mjs` | Runs `stellar-forge dev reseed` using env defaults |
| `node scripts/release.mjs --plan` | Wraps release plan/deploy/verify/env-export/aliases-sync |
| `node workers/events/ingest-events.mjs <resource> --once` | Event worker stub for project-specific ingestion |

All wrappers honor `STELLAR_FORGE_BIN` if you want them to invoke a different binary name.
