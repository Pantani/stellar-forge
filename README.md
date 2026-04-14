# stellar-forge

`stellar-forge` is a Rust CLI that turns a Stellar project into a manifest-driven workspace.
It keeps project intent in `stellarforge.toml`, persists deployment state in
`stellarforge.lock.json`, generates app scaffolds around contracts, and shells out to the
official `stellar` CLI for chain-facing work.

Examples in this repository use `stellar forge ...`. If plugin discovery is not available on your
machine yet, replace every example with `stellar-forge ...`.

## Documentation

Start here:

- [Documentation index](docs/README.md)
- [Command reference](docs/command-reference.md)
- [Manifest and state reference](docs/manifest-reference.md)
- [Deployment guide](docs/deployment-guide.md)

Official Stellar references linked from this README:

- [Install Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli)
- [Stellar CLI plugins](https://developers.stellar.org/docs/tools/cli/plugins)
- [Contract lifecycle cookbook](https://developers.stellar.org/docs/tools/cli/cookbook/contract-lifecycle)
- [Working with assets and payments](https://developers.stellar.org/docs/tools/cli/cookbook/payments-and-assets)

## What The CLI Owns

`stellar-forge` is opinionated about project structure, not about replacing the Stellar toolchain.
The split is:

- `stellar-forge` owns project scaffolding, manifest parsing, validation, dry runs, lockfile
  updates, release planning, helper scripts, and generated API/frontend/event files.
- `stellar` owns contract build/deploy/invoke, key management, alias management, local quickstart,
  and other network primitives.

That division keeps the Rust code small and makes the generated commands easy to audit with
`--dry-run` or `--json`.

## What You Get

The CLI currently covers:

- project bootstrap with templates for contract-only, fullstack, issuer-wallet, merchant-checkout,
  rewards-loyalty, API-only, and multi-contract projects
- project adoption from an existing Scaffold-style Stellar workspace
- wallet and identity flows for classic accounts, SEP-7 links, relayed payments, and smart-wallet
  scaffolding
- token flows for classic assets, SAC wrappers, and contract-token projects
- contract flows for build, deploy, invoke, bindings, TTL management, fetch, and spec/info views
- generated API, OpenAPI, relayer, frontend, and event-ingestion scaffolds
- release planning, deploy, verify, env export, alias sync, and registry-oriented deploy flows
- diagnostics for dependencies, manifest health, local layout, compatibility drift, and network
  reachability

## Requirements

The exact command you use to install dependencies may change over time, so prefer the official
Stellar docs for the external toolchain and use this README for how `stellar-forge` expects those
tools to be present.

| Dependency | Required when | Notes |
| --- | --- | --- |
| Rust stable (`cargo`, `rustc`) | Always to build this repo; also required when the project declares contracts | Used to build `stellar-forge` itself and contract workspaces |
| Official `stellar` CLI | Required for nearly every chain-facing command | `stellar-forge` shells out to it for build, deploy, wallets, aliases, quickstart, and events |
| Node.js | Required for generated API/frontend projects | Needed for `apps/api`, `apps/web`, helper scripts, and event worker scaffolds |
| Package manager (`pnpm` by default) | Required when API/frontend scaffolds are enabled | Can be changed with `project.package_manager` |
| Docker | Required for local network workflows | `dev up/down/reset/logs` call `stellar container ... local` |
| `sqlite3` | Required for persisted event backfill and cursor reset | `events backfill` stores imported events locally |
| `stellar-registry` | Optional | Used only when `stellar registry ...` is unavailable and registry flows are needed |

## Install

### Build locally

```bash
cargo build --release
./target/release/stellar-forge doctor
```

### Install into Cargo's bin directory

```bash
cargo install --path .
stellar-forge doctor
```

### Use it as a `stellar forge` plugin

Once the `stellar-forge` binary is on `PATH`, the official Stellar CLI can discover it as the
`forge` plugin:

```bash
stellar plugin ls
stellar forge doctor
```

If `forge` does not appear in `stellar plugin ls`, keep using `stellar-forge ...` directly until
plugin discovery is fixed on your machine. The official plugin model is documented in
[Stellar CLI plugins](https://developers.stellar.org/docs/tools/cli/plugins).

## Quick Start

### 1. Create a new project

```bash
stellar forge init hello-stellar --template fullstack --network testnet
cd hello-stellar
```

Useful variants:

```bash
stellar forge init rewards-app --template rewards-loyalty
stellar forge init service-only --template api-only --no-install
stellar forge init contracts-only --template multi-contract --contracts 2 --no-api
```

### 2. Check the environment

```bash
stellar forge doctor
stellar forge project validate
stellar forge project info
```

### 3. Start local development

```bash
stellar forge dev up
stellar forge dev reseed --network local
stellar forge dev status
```

`dev up` writes `.env.generated` with local RPC and Horizon endpoints. `dev reseed` rehydrates the
declared identities, tokens, contracts, event cursor state, and generated env outputs for the
selected network.

### 4. Build and deploy contracts

```bash
stellar forge contract build
stellar forge release plan testnet
stellar forge release deploy testnet
stellar forge release verify testnet
```

### 5. Run generated apps

For projects with API and frontend scaffolds:

```bash
pnpm --dir apps/api install
pnpm --dir apps/api dev
pnpm --dir apps/web install
pnpm --dir apps/web dev
```

Swap `pnpm` for `npm`, `yarn`, or `bun` if `project.package_manager` says otherwise.

## Generated Project Layout

The generated workspace revolves around these paths:

| Path | Purpose |
| --- | --- |
| `stellarforge.toml` | Declarative source of truth for the project |
| `stellarforge.lock.json` | Materialized deploy state per environment |
| `.env.example` | Baseline variables derived from the manifest |
| `.env.generated` | Environment values exported from actual deployed state |
| `contracts/` | Contract workspaces |
| `packages/` | Generated bindings |
| `apps/api` | Generated API scaffold |
| `apps/web` | Generated frontend scaffold |
| `workers/events` | Event ingestion scripts and cursor snapshot |
| `dist/` | Deploy snapshots, registry artifacts, fetched Wasm files |
| `scripts/doctor.mjs` | Runs `stellar-forge doctor` using env defaults |
| `scripts/reseed.mjs` | Runs `stellar-forge dev reseed` using env defaults |
| `scripts/release.mjs` | Wrapper for `release plan|deploy|verify|env export|aliases sync` |

## Command Groups

| Group | Main responsibility |
| --- | --- |
| `init` | Bootstrap a new workspace from a template |
| `project` | Inspect, validate, sync, extend, or adopt a workspace |
| `doctor` | Check dependencies, manifest health, generated files, and network reachability |
| `dev` | Control the local quickstart network and reseed project state |
| `contract` | Build, deploy, invoke, inspect, bind, fetch, and manage TTL |
| `token` | Create and operate classic assets, SAC wrappers, and contract tokens |
| `wallet` | Create/fund/list wallets, inspect balances, build payments, and create SEP-7 payloads |
| `api` | Generate or refresh the API scaffold and OpenAPI output |
| `events` | Watch events, backfill recent history, and manage cursors |
| `release` | Plan, deploy, verify, export env, sync aliases, and manage registry flows |

The full syntax and examples live in [docs/command-reference.md](docs/command-reference.md).

## Typical Workflows

### Project hygiene

```bash
stellar forge project validate
stellar forge project sync
stellar forge --json project info
```

Use this loop whenever you edit `stellarforge.toml`.

### Wallets and payments

```bash
stellar forge wallet create alice --fund
stellar forge wallet balances alice
stellar forge wallet pay --from alice --to bob --asset XLM --amount 10
stellar forge wallet pay --from alice --to bob --asset points --amount 25 --sep7
stellar forge wallet receive alice --sep7 --asset points
```

### Tokens

```bash
stellar forge token create points \
  --mode asset \
  --issuer issuer \
  --distribution treasury \
  --with-sac \
  --initial-supply 1000000

stellar forge token mint points --to alice --amount 100
stellar forge token transfer points --from alice --to bob --amount 10
stellar forge token sac deploy points
```

### Contracts

```bash
stellar forge contract new escrow --template escrow
stellar forge contract build escrow --optimize
stellar forge contract deploy escrow --env testnet
stellar forge contract bind escrow --lang typescript,python
stellar forge contract fetch escrow
```

### Events

```bash
stellar forge api events init
stellar forge events watch contract rewards
stellar forge events backfill contract:rewards --count 200
stellar forge events cursor ls
```

### Release and deployment

```bash
stellar forge release plan testnet
stellar forge release deploy testnet
stellar forge release verify testnet
stellar forge release env export testnet
stellar forge release aliases sync testnet
```

The release flow is described in depth in [docs/deployment-guide.md](docs/deployment-guide.md).

## Global Flags

These apply to most commands:

| Flag | Meaning |
| --- | --- |
| `--manifest <path>` | Use a manifest outside the default `./stellarforge.toml` |
| `--cwd <path>` | Change the working directory the CLI resolves from |
| `--network <name>` | Override the active network from `defaults.network` |
| `--identity <name>` | Override the active identity from `defaults.identity` |
| `--json` | Emit a structured report instead of human-readable output |
| `--quiet` | Reduce non-essential output |
| `--verbose` / `-vv` | Increase verbosity |
| `--dry-run` | Plan commands without mutating files, hitting the network, or spawning CLI actions |
| `--yes` | Skip confirmations in flows that support prompts |

`--json` reports include structured fields such as `status`, `action`, `checks`, `commands`,
`artifacts`, `next`, and `data`, which makes the tool easy to script in CI.

## Deploying To Real Networks

The short version:

1. make sure the target network exists in `stellarforge.toml`
2. build and validate locally
3. run `stellar forge release plan <env>`
4. deploy with `stellar forge release deploy <env>`
5. verify with `stellar forge release verify <env>`
6. export runtime env with `stellar forge release env export <env>`
7. optionally sync aliases with `stellar forge release aliases sync <env>`

Important behavior to know:

- if `[release.<env>]` is missing, the release flow falls back to all declared contracts and tokens
- `.env.generated` and `dist/deploy.<env>.json` are treated as deploy artifacts
- `release deploy pubnet` is intentionally blocked unless `--confirm-mainnet` is passed
- registry workflows can use either `stellar registry ...` or the standalone `stellar-registry`
  binary, controlled by `STELLAR_FORGE_REGISTRY_MODE`

See [docs/deployment-guide.md](docs/deployment-guide.md) for the full checklist, examples, and
artifact model.

## Repository Development

This repository uses the following quality gates locally and in CI:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo audit
```

GitHub Actions runs the same checks from `.github/workflows/ci.yml`.

## Community and Contribution

- contribution workflow: [CONTRIBUTING.md](CONTRIBUTING.md)
- support and troubleshooting channels: [SUPPORT.md](SUPPORT.md)
- responsible disclosure process: [SECURITY.md](SECURITY.md)
- expected project behavior: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

GitHub issue forms, a pull request template, Dependabot updates, and CI all live in `.github/` so
new changes follow the same path from report to review to validation.

## Troubleshooting

### `stellar forge` does not appear in `stellar plugin ls`

Make sure the `stellar-forge` binary is on `PATH`. Until plugin discovery works, use
`stellar-forge ...` directly.

### Commands fail because `stellar` is missing

Install the official `stellar` CLI first, then rerun `stellar forge doctor deps`.

### `dev up` fails

`dev up` expects `[networks.local]` to exist and to have `kind = "local"`. It also needs Docker
because it shells out to `stellar container start local`.

### `events backfill` fails immediately

That command needs the API scaffold and persisted storage. Run `stellar forge events ingest init`
first, and make sure `sqlite3` is installed locally.

### A token cannot be watched or backfilled as events

Token event flows require a contract wrapper. Deploy a SAC with `stellar forge token sac deploy
<name>` or use a contract token.

### `release verify` warns that the network may have reset

Test environments can lose state. When IDs in `stellarforge.lock.json` no longer resolve, run a
fresh `dev reseed` or redeploy the target environment, then export env again.
