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

Quick navigation by task:

| I want to... | Start here | Then read |
| --- | --- | --- |
| bootstrap a new workspace | [this README](README.md#quick-start) | [Command reference](docs/command-reference.md#init) |
| understand `stellarforge.toml` and the lockfile | [Manifest and state reference](docs/manifest-reference.md) | [Deployment guide](docs/deployment-guide.md) |
| look up exact syntax and flags | [Command reference](docs/command-reference.md) | [README workflows](README.md#typical-workflows) |
| ship to testnet or pubnet | [Deployment guide](docs/deployment-guide.md) | [Manifest and state reference](docs/manifest-reference.md#generated-files-and-artifacts) |
| debug local setup or generated files | [README troubleshooting](README.md#troubleshooting) | [Command reference](docs/command-reference.md#doctor) |
| work with batch payouts, airdrops, or smart wallets | [README workflows](README.md#typical-workflows) | [Command reference](docs/command-reference.md#wallet) |

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
- wallet and identity flows for classic accounts, SEP-7 links, relayed payments, smart-wallet
  scaffolding, onboarding, materialization, controller rotation, policy simulation, and batch-file
  validation and preview
- token flows for classic assets, SAC wrappers, contract-token projects, and token-scoped airdrop
  validation helpers
- contract flows for build, format, lint, deploy, invoke, bindings, TTL management, fetch, and spec/info views
- manifest-driven scenario rehearsal through `scenario run` and `scenario test`, including
  assertion-aware preview checks
- generated API, OpenAPI, relayer, frontend, and event-ingestion scaffolds
- frontend smoke validation through `project smoke`, `project smoke --browser`, and the generated smoke runners
- release planning, status, diff, deploy, verify, history management, env export, alias sync, and
  registry-oriented deploy flows
- diagnostics for dependencies, manifest health, local layout, compatibility drift, and network
  reachability, plus development snapshot save/load flows with local history fallback

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

Template chooser:

| Template | Best when you want | Typical next command |
| --- | --- | --- |
| `minimal-contract` | one contract and little else | `stellar forge contract build` |
| `fullstack` | contract + API + frontend from day one | `stellar forge project smoke` |
| `issuer-wallet` | an issuing identity and treasury flows | `stellar forge token create points --mode asset --with-sac` |
| `merchant-checkout` | payment-oriented scaffolding and wallet UX | `stellar forge wallet pay --from treasury --to alice --asset points --amount 10` |
| `rewards-loyalty` | a concrete loyalty demo with release defaults | `stellar forge release plan testnet` |
| `api-only` | backend routes and OpenAPI without frontend/contract defaults | `stellar forge api generate contract <name>` |
| `multi-contract` | multiple contracts with shared workspace management | `stellar forge contract build` |

### 2. Check the environment

```bash
stellar forge doctor
stellar forge doctor --out dist/doctor.json
stellar forge project validate
stellar forge project validate --out dist/project.validate.json
stellar forge project info
stellar forge project info --out dist/project.info.json
```

### 3. Start local development

```bash
stellar forge dev up
stellar forge dev reset
stellar forge dev reseed --network local
stellar forge dev status
stellar forge dev fund alice
stellar forge dev watch --once
stellar forge dev events rewards
stellar forge dev logs
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
pnpm --dir apps/web smoke:ui
pnpm --dir apps/web smoke:browser:build
pnpm --dir apps/web smoke:browser:install
pnpm --dir apps/web smoke:browser:run
pnpm --dir apps/web smoke:browser
stellar forge project smoke
stellar forge project smoke --browser
```

Swap `pnpm` for `npm`, `yarn`, or `bun` if `project.package_manager` says otherwise.
The browser smoke runner now splits into build, install, and run steps. The default
`smoke:browser` command still runs the full flow, but it first checks the Playwright cache and skips
reinstalling Chromium when the matching browser bundle is already present.
When port `4173` is busy, set `STELLAR_FORGE_BROWSER_SMOKE_PORT=<port>` before running it.

### 6. Follow the day-zero loop

Once the scaffold exists, this sequence covers the most common first hour:

```bash
stellar forge doctor
stellar forge project validate
stellar forge project sync
stellar forge dev up
stellar forge dev reseed --network local
stellar forge project smoke
stellar forge --dry-run release plan testnet
```

What that does:

- `doctor` checks the machine and the workspace shape
- `project validate` catches manifest and generated-file problems early
- `project sync` refreshes derived files after edits
- `dev up` and `dev reseed` give you a repeatable local sandbox
- `project smoke` validates the generated frontend/API integration path
- `release plan` tells you what would change on a real network before it changes anything

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
| `doctor` | Check dependencies, manifest health, generated files, network reachability, and repair managed drift |
| `dev` | Control the local quickstart network, reseed project state, and save or restore local snapshots |
| `scenario` | Run or preview manifest-declared rehearsal flows that compose existing project, wallet, token, contract, and release steps |
| `contract` | Build, format, lint, deploy, invoke, inspect, bind, fetch, and manage TTL |
| `token` | Create and operate classic assets, SAC wrappers, contract tokens, airdrop reports, and scenario-friendly token flows |
| `wallet` | Create/fund/list wallets, inspect balances, build payments, validate, preview, and report batch files, manage smart-wallet policy flows, and create SEP-7 payloads |
| `api` | Generate or refresh the API scaffold and OpenAPI output |
| `events` | Inspect event status, watch events, backfill recent history, and manage cursors |
| `release` | Plan, diff, inspect history, prune archived snapshots, verify, roll back local release metadata, export env, sync aliases, and manage registry flows |

The full syntax and examples live in [docs/command-reference.md](docs/command-reference.md).

## Typical Workflows

### Project hygiene

```bash
stellar forge project validate
stellar forge doctor fix
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
stellar forge wallet sep7 payment --from alice --to bob --asset points --amount 25
stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json
stellar forge wallet batch-validate --from treasury --asset points --file payouts.json
stellar forge wallet batch-preview --from treasury --asset points --file payouts.json
stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json
stellar forge wallet smart create guardian --mode ed25519
stellar forge wallet smart scaffold guardian
stellar forge wallet smart onboard checkout-passkey
stellar forge wallet smart materialize checkout-passkey
stellar forge wallet smart controller rotate checkout-passkey alice
stellar forge wallet smart policy info guardian
stellar forge wallet smart policy allow guardian alice --build-only
stellar forge wallet smart policy revoke guardian alice --build-only
stellar forge wallet smart policy set-daily-limit guardian 1250 --build-only
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
stellar forge token trust points alice
stellar forge token freeze points alice
stellar forge token unfreeze points alice
stellar forge token clawback points alice 10
stellar forge token airdrop points --from treasury --file rewards.csv --format csv
stellar forge token airdrop-validate points --file rewards.csv --format csv
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv
stellar forge token sac id points
stellar forge token sac deploy points
```

### Contracts

```bash
stellar forge contract new escrow --template escrow
stellar forge contract format escrow --check
stellar forge contract lint escrow
stellar forge contract build escrow --optimize
stellar forge contract deploy escrow --env testnet
stellar forge contract bind escrow --lang typescript,python
stellar forge contract fetch escrow
stellar forge contract fetch escrow --out ./tmp/escrow.wasm
```

### API and services

```bash
stellar forge api init
stellar forge api generate contract rewards
stellar forge api openapi export
stellar forge api relayer init
```

### Events

```bash
stellar forge api events init
stellar forge events status
stellar forge events export --path dist/events.json
stellar forge events replay --path dist/events.json
stellar forge events watch contract rewards
stellar forge events ingest init
stellar forge events backfill contract:rewards --count 200
stellar forge events cursor ls
stellar forge events cursor reset testnet:contract:rewards
```

### Release and deployment

```bash
stellar forge release plan testnet
stellar forge release status testnet
stellar forge release diff testnet
stellar forge release deploy testnet
stellar forge release history testnet
stellar forge release inspect testnet
stellar forge release verify testnet
stellar forge release prune testnet --keep 3
stellar forge release rollback testnet --to dist/history/deploy.testnet.<timestamp>.json
stellar forge release env export testnet
stellar forge release aliases sync testnet
stellar forge --network testnet release registry publish rewards
stellar forge --network testnet release registry deploy rewards
```

### Diagnostics

```bash
stellar forge doctor env
stellar forge doctor deps
stellar forge doctor project
stellar forge doctor network local
stellar forge doctor fix --scope release
```

The release flow is described in depth in [docs/deployment-guide.md](docs/deployment-guide.md).

## Input File Examples

Batch and policy commands are much easier to use when you keep a few canonical file shapes around.

### Wallet batch JSON

Use this with `wallet batch-pay`, `wallet batch-report`, `wallet batch-validate`,
`wallet batch-preview`, `wallet batch-summary`, `wallet batch-reconcile`, and
`wallet batch-resume`.

```json
[
  { "to": "alice", "amount": "10", "asset": "XLM" },
  { "to": "bob", "amount": "25", "asset": "points" },
  { "to": "carol", "amount": "5" }
]
```

When `asset` is omitted on a row, the command falls back to the command-level `--asset`.

### Wallet batch CSV

```csv
to,amount,asset
alice,10,XLM
bob,25,points
carol,5,
```

Use `--format csv` when the filename does not already make the format obvious.

### Token airdrop CSV

Token airdrop commands reuse the batch-payment format, but the token name supplies the asset:

```csv
to,amount
alice,10
bob,20
carol,5
```

Example:

```bash
stellar forge token airdrop points --from treasury --file rewards.csv --format csv
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json
```

### Smart-wallet policy TOML

Use this with `wallet smart policy apply` and `wallet smart policy simulate`:

```toml
source = "alice"
daily_limit = 1250
allow = ["treasury", "issuer"]
revoke = ["legacy-signer"]
build_only = true
```

JSON works too:

```json
{
  "source": "alice",
  "daily_limit": 1250,
  "allow": ["treasury", "issuer"],
  "revoke": ["legacy-signer"],
  "build_only": true
}
```

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

## Reports And Artifacts

Most commands support three useful modes:

- human output only: good for interactive use
- `--json`: print the structured report to stdout
- `--out <path>`: persist the structured report to disk, usually alongside the normal stdout mode

Common patterns:

```bash
stellar forge --json project validate
stellar forge project validate --out dist/project.validate.json
stellar forge --json release plan testnet --out dist/release.plan.json
stellar forge events export --path dist/events.json --out dist/events.export.json
```

Two details are worth remembering:

- for most commands, `--out` controls the report file path
- for `contract fetch`, `--out` controls the fetched Wasm artifact path itself

Useful report fields:

| Field | Meaning |
| --- | --- |
| `status` | overall outcome such as `ok`, `warn`, or `error` |
| `checks` | typed validation or verification checks |
| `commands` | underlying commands the CLI ran or would run |
| `artifacts` | files written or touched by the command |
| `next` | suggested follow-up commands |
| `data` | command-specific payload that is easiest to script against |

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

## What To Commit

When a change is intentional, commit the declarative input and the materialized outputs together.

Typical release-related commit set:

- `stellarforge.toml`
- `stellarforge.lock.json`
- `.env.generated` when your workflow tracks generated runtime values
- `dist/deploy.<env>.json`
- generated files changed by `project sync`, `api generate`, `api events init`, `api relayer init`, or smart-wallet scaffolding

Typical documentation-friendly commit set after workspace changes:

- updated docs under `docs/`
- generated README or helper scripts if template behavior changed
- focused test updates when new commands, flags, or outputs were documented

## New Commands

The latest smart-wallet, batch, project-smoke, release, doctor, and events additions cover local
provisioning, policy drift, batch-file reporting, report exports, and event-store checks:

```bash
stellar forge project smoke --out dist/project.smoke.json
stellar forge doctor audit --out dist/doctor.audit.json
stellar forge doctor fix --scope release --out dist/doctor.fix.json
stellar forge project info --out dist/project.info.json
stellar forge project validate --out dist/project.validate.json
stellar forge project sync --out dist/project.sync.json
stellar forge project adopt scaffold --out dist/project.adopt.json
stellar forge project add contract escrow --template escrow --out dist/project.add.contract.json
stellar forge project add api --out dist/project.add.api.json
stellar forge project add frontend --framework react-vite --out dist/project.add.frontend.json
stellar forge dev up --out dist/dev.up.json
stellar forge dev down --out dist/dev.down.json
stellar forge dev reset --out dist/dev.reset.json
stellar forge dev reseed --out dist/dev.reseed.json
stellar forge --network testnet --dry-run dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/dev.fund.json
stellar forge --dry-run --network local dev watch --once --out dist/dev.watch.json
stellar forge dev events rewards --out dist/dev.events.json
stellar forge --dry-run dev logs --out dist/dev.logs.json
stellar forge dev status --out dist/dev.status.json
stellar forge contract new escrow --template escrow --out dist/contract.new.json
stellar forge contract build rewards --out dist/contract.build.json
stellar forge contract deploy rewards --out dist/contract.deploy.json
stellar forge contract call rewards award_points --out dist/contract.call.json -- --member alice --amount 25
stellar forge contract bind rewards --lang typescript --out dist/contract.bind.json
stellar forge contract info credits --out dist/contract.info.json
stellar forge contract spec rewards --out dist/contract.spec.json
stellar forge contract ttl extend rewards --out dist/contract.ttl.extend.json
stellar forge contract ttl restore rewards --out dist/contract.ttl.restore.json
stellar forge --dry-run --network testnet token create credits --mode contract --metadata-name "Store Credit" --initial-supply 25 --out dist/token.create.json
stellar forge --dry-run --network testnet token mint credits --to alice --amount 10 --from issuer --out dist/token.mint.json
stellar forge token burn points --amount 5 --from treasury --out dist/token.burn.json
stellar forge token transfer points --to alice --amount 10 --from treasury --out dist/token.transfer.json
stellar forge token trust points alice --out dist/token.trust.json
stellar forge token freeze points alice --out dist/token.freeze.json
stellar forge token unfreeze points alice --out dist/token.unfreeze.json
stellar forge token clawback points alice 1 --out dist/token.clawback.json
stellar forge --network testnet token sac id points --out dist/token.sac.id.json
stellar forge --network testnet token sac deploy points --out dist/token.sac.deploy.json
stellar forge --network testnet token contract init credits --out dist/token.contract.init.json
stellar forge token info points --out dist/token.info.json
stellar forge token balance points --holder alice --out dist/token.balance.json
stellar forge wallet create bob --fund --out dist/wallet.create.json
stellar forge wallet fund alice --out dist/wallet.fund.json
stellar forge wallet trust alice points --out dist/wallet.trust.json
stellar forge wallet pay --from treasury --to alice --asset points --amount 10 --out dist/wallet.pay.json
stellar forge wallet sep7 payment --from treasury --to alice --asset points --amount 10 --out dist/wallet.sep7.payment.json
stellar forge --network testnet wallet sep7 contract-call rewards award_points --out dist/wallet.sep7.contract-call.json -- --member alice --amount 25
stellar forge wallet ls --out dist/wallet.ls.json
stellar forge wallet address alice --out dist/wallet.address.json
stellar forge wallet balances alice --out dist/wallet.balances.json
stellar forge wallet receive alice --sep7 --asset points --out dist/wallet.receive.json
stellar forge wallet smart create sentinel --mode ed25519 --out dist/wallet.smart.create.json
stellar forge wallet smart scaffold guardian --out dist/wallet.smart.scaffold.json
stellar forge wallet smart info guardian --out dist/wallet.smart.info.json
stellar forge wallet smart onboard checkout-passkey --out dist/wallet.smart.onboard.json
stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/wallet.smart.provision.json
stellar forge wallet smart materialize checkout-passkey --out dist/wallet.smart.materialize.json
stellar forge wallet smart controller rotate checkout-passkey alice --out dist/wallet.smart.controller.rotate.json
stellar forge wallet smart policy info guardian --out dist/wallet.smart.policy.info.json
stellar forge wallet smart policy set-daily-limit sentinel 1250 --build-only --out dist/wallet.smart.policy.set-daily-limit.json
stellar forge wallet smart policy allow sentinel alice --build-only --out dist/wallet.smart.policy.allow.json
stellar forge wallet smart policy revoke sentinel alice --build-only --out dist/wallet.smart.policy.revoke.json
stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json
stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json
stellar forge wallet smart policy apply checkout-passkey --file policy.toml --out dist/wallet.smart.policy.apply.json
stellar forge api init --out dist/api.init.json
stellar forge api generate contract rewards --out dist/api.generate.contract.json
stellar forge api generate token points --out dist/api.generate.token.json
stellar forge api openapi export --out dist/api.openapi.json
stellar forge api events init --out dist/api.events.init.json
stellar forge api relayer init --out dist/api.relayer.init.json
stellar forge events export --path dist/events.json --out dist/events.export.json
stellar forge events replay --path dist/events.json --out dist/events.replay.json
stellar forge events watch contract rewards --out dist/events.watch.json
stellar forge events ingest init --out dist/events.ingest.init.json
stellar forge events backfill contract:rewards --count 200 --out dist/events.backfill.json
stellar forge events status --out dist/events.status.json
stellar forge events cursor ls --out dist/events.cursor.json
stellar forge events cursor reset testnet:contract:rewards --out dist/events.cursor.reset.json
stellar forge doctor --out dist/doctor.json
stellar forge release deploy testnet --out dist/release.deploy.json
stellar forge release status testnet --out dist/release.status.json
stellar forge release drift testnet --out dist/release.drift.json
stellar forge release diff testnet --out dist/release.diff.json
stellar forge release history testnet --out dist/release.history.json
stellar forge release inspect testnet --out dist/release.inspect.json
stellar forge release rollback testnet --out dist/release.rollback.json
stellar forge release prune testnet --keep 3 --out dist/release.prune.json
stellar forge release plan testnet --out dist/release.plan.json
stellar forge release verify testnet --out dist/release.verify.json
stellar forge release env export testnet --out dist/release.env.json
stellar forge release aliases sync testnet --out dist/release.aliases.json
stellar forge --network testnet release registry publish rewards --out dist/release.registry.publish.json
stellar forge --network testnet release registry deploy rewards --out dist/release.registry.deploy.json
stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
stellar forge wallet smart provision checkout-passkey --fund
stellar forge wallet smart policy sync checkout-passkey
stellar forge wallet smart policy diff checkout-passkey
stellar forge release drift testnet
stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json --out dist/payouts.pay.json
stellar forge wallet batch-validate --from treasury --asset points --file payouts.json --out dist/payouts.validate.json
stellar forge wallet batch-preview --from treasury --asset points --file payouts.csv --format csv --out dist/payouts.preview.json
stellar forge wallet batch-summary --from treasury --asset points --file payouts.json --out dist/payouts.summary.json
stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv
stellar forge events status
stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json
stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.reconcile.json
stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.resume.json
stellar forge token airdrop points --from treasury --file rewards.csv --format csv --out dist/airdrop.json
stellar forge token airdrop-validate points --file rewards.csv --format csv --out dist/airdrop.validate.json
stellar forge token airdrop-preview points --from treasury --file rewards.json --out dist/airdrop.preview.json
stellar forge token airdrop-summary points --file rewards.csv --format csv --out dist/airdrop.summary.json
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.reconcile.json
stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.resume.json
stellar forge doctor env --out dist/doctor.env.json
stellar forge doctor deps --out dist/doctor.deps.json
stellar forge doctor project --out dist/doctor.project.json
stellar forge doctor network local --out dist/doctor.network.json
```

Additional command surfaces:

```bash
stellar forge wallet smart policy apply checkout-passkey --file policy.toml
stellar forge project adopt scaffold --out dist/project.adopt.json
stellar forge project add contract escrow --template escrow --out dist/project.add.contract.json
stellar forge project add api --out dist/project.add.api.json
stellar forge project add frontend --framework react-vite --out dist/project.add.frontend.json
stellar forge dev up --out dist/dev.up.json
stellar forge dev down --out dist/dev.down.json
stellar forge dev reset --out dist/dev.reset.json
stellar forge dev reseed --out dist/dev.reseed.json
stellar forge --network testnet --dry-run dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/dev.fund.json
stellar forge --dry-run --network local dev watch --once --out dist/dev.watch.json
stellar forge dev events rewards --out dist/dev.events.json
stellar forge --dry-run dev logs --out dist/dev.logs.json
stellar forge contract new escrow --template escrow --out dist/contract.new.json
stellar forge contract build rewards --out dist/contract.build.json
stellar forge contract deploy rewards --out dist/contract.deploy.json
stellar forge contract call rewards award_points --out dist/contract.call.json -- --member alice --amount 25
stellar forge contract bind rewards --lang typescript --out dist/contract.bind.json
stellar forge contract ttl extend rewards --out dist/contract.ttl.extend.json
stellar forge contract ttl restore rewards --out dist/contract.ttl.restore.json
stellar forge --dry-run --network testnet token create credits --mode contract --metadata-name "Store Credit" --initial-supply 25 --out dist/token.create.json
stellar forge --dry-run --network testnet token mint credits --to alice --amount 10 --from issuer --out dist/token.mint.json
stellar forge token burn points --amount 5 --from treasury --out dist/token.burn.json
stellar forge token transfer points --to alice --amount 10 --from treasury --out dist/token.transfer.json
stellar forge token trust points alice --out dist/token.trust.json
stellar forge token freeze points alice --out dist/token.freeze.json
stellar forge token unfreeze points alice --out dist/token.unfreeze.json
stellar forge token clawback points alice 1 --out dist/token.clawback.json
stellar forge --network testnet token sac id points --out dist/token.sac.id.json
stellar forge --network testnet token sac deploy points --out dist/token.sac.deploy.json
stellar forge --network testnet token contract init credits --out dist/token.contract.init.json
stellar forge api init --out dist/api.init.json
stellar forge api generate contract rewards --out dist/api.generate.contract.json
stellar forge api generate token points --out dist/api.generate.token.json
stellar forge api openapi export --out dist/api.openapi.json
stellar forge api events init --out dist/api.events.init.json
stellar forge api relayer init --out dist/api.relayer.init.json
stellar forge wallet create bob --fund --out dist/wallet.create.json
stellar forge wallet fund alice --out dist/wallet.fund.json
stellar forge wallet trust alice points --out dist/wallet.trust.json
stellar forge wallet pay --from treasury --to alice --asset points --amount 10 --out dist/wallet.pay.json
stellar forge wallet sep7 payment --from treasury --to alice --asset points --amount 10 --out dist/wallet.sep7.payment.json
stellar forge --network testnet wallet sep7 contract-call rewards award_points --out dist/wallet.sep7.contract-call.json -- --member alice --amount 25
stellar forge release plan testnet --out dist/release.plan.json
stellar forge release deploy testnet --out dist/release.deploy.json
stellar forge release verify testnet --out dist/release.verify.json
stellar forge release aliases sync testnet --out dist/release.aliases.json
stellar forge release env export testnet --out dist/release.env.json
stellar forge --network testnet release registry publish rewards --out dist/release.registry.publish.json
stellar forge --network testnet release registry deploy rewards --out dist/release.registry.deploy.json
stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json --out dist/payouts.pay.json
stellar forge wallet batch-validate --from treasury --asset points --file payouts.json --out dist/payouts.validate.json
stellar forge wallet batch-preview --from treasury --asset points --file payouts.csv --format csv --out dist/payouts.preview.json
stellar forge wallet batch-summary --from treasury --asset points --file payouts.json --out dist/payouts.summary.json
stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json --out dist/payouts.report.json
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv --out dist/airdrop.report.json
stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.reconcile.json
stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.resume.json
stellar forge token airdrop points --from treasury --file rewards.csv --format csv --out dist/airdrop.json
stellar forge token airdrop-validate points --file rewards.csv --format csv --out dist/airdrop.validate.json
stellar forge token airdrop-preview points --from treasury --file rewards.json --out dist/airdrop.preview.json
stellar forge token airdrop-summary points --file rewards.csv --format csv --out dist/airdrop.summary.json
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.reconcile.json
stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.resume.json
stellar forge events export
stellar forge events replay
stellar forge events watch contract rewards --out dist/events.watch.json
stellar forge events ingest init --out dist/events.ingest.init.json
stellar forge events cursor reset testnet:contract:rewards --out dist/events.cursor.reset.json
stellar forge doctor audit
stellar forge doctor env --out dist/doctor.env.json
stellar forge doctor deps --out dist/doctor.deps.json
stellar forge doctor project --out dist/doctor.project.json
stellar forge doctor network local --out dist/doctor.network.json
stellar forge doctor fix --scope events
stellar forge doctor fix --scope release
stellar forge project smoke --out dist/project.smoke.json
stellar forge doctor audit --out dist/doctor.audit.json
stellar forge doctor fix --scope release --out dist/doctor.fix.json
stellar forge release status testnet --out dist/release.status.json
stellar forge release drift testnet --out dist/release.drift.json
stellar forge release diff testnet --out dist/release.diff.json
stellar forge release history testnet --out dist/release.history.json
stellar forge release inspect testnet --out dist/release.inspect.json
stellar forge project info --out dist/project.info.json
stellar forge project validate --out dist/project.validate.json
stellar forge project sync --out dist/project.sync.json
stellar forge dev status --out dist/dev.status.json
stellar forge contract info credits --out dist/contract.info.json
stellar forge contract spec rewards --out dist/contract.spec.json
stellar forge token info points --out dist/token.info.json
stellar forge token balance points --holder alice --out dist/token.balance.json
stellar forge wallet ls --out dist/wallet.ls.json
stellar forge wallet address alice --out dist/wallet.address.json
stellar forge wallet balances alice --out dist/wallet.balances.json
stellar forge wallet receive alice --sep7 --asset points --out dist/wallet.receive.json
stellar forge wallet smart create sentinel --mode ed25519 --out dist/wallet.smart.create.json
stellar forge wallet smart scaffold guardian --out dist/wallet.smart.scaffold.json
stellar forge wallet smart info guardian --out dist/wallet.smart.info.json
stellar forge wallet smart onboard checkout-passkey --out dist/wallet.smart.onboard.json
stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/wallet.smart.provision.json
stellar forge wallet smart materialize checkout-passkey --out dist/wallet.smart.materialize.json
stellar forge wallet smart controller rotate checkout-passkey alice --out dist/wallet.smart.controller.rotate.json
stellar forge wallet smart policy info guardian --out dist/wallet.smart.policy.info.json
stellar forge wallet smart policy set-daily-limit sentinel 1250 --build-only --out dist/wallet.smart.policy.set-daily-limit.json
stellar forge wallet smart policy allow sentinel alice --build-only --out dist/wallet.smart.policy.allow.json
stellar forge wallet smart policy revoke sentinel alice --build-only --out dist/wallet.smart.policy.revoke.json
stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json
stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json
stellar forge wallet smart policy apply checkout-passkey --file policy.toml --out dist/wallet.smart.policy.apply.json
stellar forge events export --path dist/events.json --out dist/events.export.json
stellar forge events replay --path dist/events.json --out dist/events.replay.json
stellar forge events backfill contract:rewards --count 200 --out dist/events.backfill.json
stellar forge events status --out dist/events.status.json
stellar forge events cursor ls --out dist/events.cursor.json
stellar forge doctor --out dist/doctor.json
stellar forge release rollback testnet --out dist/release.rollback.json
stellar forge release prune testnet --keep 3 --out dist/release.prune.json
```

## Repository Development

This repository uses the following quality gates locally and in CI:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo audit
node scripts/generated-frontend-browser-smoke.mjs
```

GitHub Actions runs the same checks from `.github/workflows/ci.yml`.
It also generates a fresh frontend scaffold and runs the frontend smoke runner against it.
The browser smoke script provisions a temporary fullstack app, installs frontend dependencies,
runs `smoke:browser:build`, `smoke:browser:install`, and `smoke:browser:run` in sequence, and
skips the Chromium download when the pinned Playwright browser bundle is already cached.

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
