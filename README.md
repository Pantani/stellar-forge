# stellar-forge

`stellar-forge` is a Rust CLI for manifest-driven Stellar workspaces.

It keeps desired project state in `stellarforge.toml`, persists observed deployment state in
`stellarforge.lock.json`, generates scaffolds around contracts, and shells out to the official
`stellar` CLI for chain-facing work.

Examples in this repository use `stellar forge ...`. If plugin discovery is not available on your
machine yet, replace every example with `stellar-forge ...`.

## What This Project Is

`stellar-forge` is an orchestration layer for Stellar projects, not a replacement for the Stellar
toolchain.

The split is:

- `stellar-forge` owns scaffolding, manifest parsing, validation, dry runs, lockfile updates,
  release planning, generated helper files, and project-oriented diagnostics
- `stellar` owns contract build/deploy/invoke, key management, alias management, local quickstart,
  and other low-level network primitives

That separation keeps the CLI focused on project structure and repeatable workflows.

## Documentation

Use this README for orientation. Use the docs folder for exact behavior.

| If you want to... | Read this |
| --- | --- |
| understand what the project does | [this README](README.md) |
| find exact command syntax and examples | [docs/command-reference.md](docs/command-reference.md) |
| understand `stellarforge.toml`, the lockfile, and generated artifacts | [docs/manifest-reference.md](docs/manifest-reference.md) |
| plan or troubleshoot deploys | [docs/deployment-guide.md](docs/deployment-guide.md) |
| choose the right doc for a task | [docs/README.md](docs/README.md) |

Official Stellar references:

- [Install Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli)
- [Stellar CLI plugins](https://developers.stellar.org/docs/tools/cli/plugins)
- [Contract lifecycle cookbook](https://developers.stellar.org/docs/tools/cli/cookbook/contract-lifecycle)
- [Working with assets and payments](https://developers.stellar.org/docs/tools/cli/cookbook/payments-and-assets)

## What You Get

The CLI currently covers:

- project bootstrap from templates such as `minimal-contract`, `fullstack`, `issuer-wallet`,
  `merchant-checkout`, `rewards-loyalty`, `api-only`, and `multi-contract`
- manifest-driven validation, synchronization, and release planning
- contract-oriented workspace flows including build, deploy, invoke, bindings, TTL, fetch,
  formatting, and linting
- classic wallet, SEP-7, relayer-aware, batch-payment, and smart-wallet workflows
- token flows for classic assets, SAC wrappers, contract tokens, and token-scoped airdrops
- generated API, OpenAPI, relayer, frontend, and event-ingestion scaffolds
- local development helpers, smoke checks, release history, drift, diff, rollback, and env export

For the full command surface, go straight to
[docs/command-reference.md](docs/command-reference.md).

## Core Files

These files define the project model:

| Path | Meaning |
| --- | --- |
| `stellarforge.toml` | declarative source of truth for the project |
| `stellarforge.lock.json` | materialized deploy state per environment |
| `.env.example` | manifest-derived defaults |
| `.env.generated` | runtime values exported from actual local or deployed state |
| `dist/deploy.<env>.json` | machine-readable release artifact |
| `workers/events/cursors.json` | local event cursor snapshot |

The relationship between these files is documented in
[docs/manifest-reference.md](docs/manifest-reference.md).

## Requirements

The exact install commands for external tools may change over time, so prefer the official Stellar
docs for those tools and use this README for how `stellar-forge` expects them to be available.

| Dependency | Required when | Notes |
| --- | --- | --- |
| Rust stable (`cargo`, `rustc`) | always to build this repo; also required when the project declares contracts | used to build `stellar-forge` itself and contract workspaces |
| official `stellar` CLI | required for most chain-facing commands | `stellar-forge` shells out to it for build, deploy, wallets, aliases, quickstart, and events |
| Node.js | required for generated API/frontend projects | used by `apps/api`, `apps/web`, helper scripts, and event workers |
| package manager (`pnpm` by default) | required when API/frontend scaffolds are enabled | can be changed with `project.package_manager` |
| Docker | required for local network workflows | `dev up/down/reset/logs` call into local container flows |
| `sqlite3` | required for persisted event backfill and cursor reset | event backfill stores imported events locally |
| `stellar-registry` | optional | used only when `stellar registry ...` is unavailable and registry flows are needed |

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
plugin discovery is working on your machine.

## Quick Start

### 1. Create a workspace

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

Template reference:

| Template | Good fit when you want |
| --- | --- |
| `minimal-contract` | one contract and very little else |
| `fullstack` | contract, API, and frontend from day one |
| `issuer-wallet` | issuer and treasury-oriented flows |
| `merchant-checkout` | payment-oriented scaffolding |
| `rewards-loyalty` | a more complete loyalty example with release defaults |
| `api-only` | generated backend without frontend or contract defaults |
| `multi-contract` | multiple contracts managed in one workspace |

### 2. Validate the environment and the manifest

```bash
stellar forge doctor
stellar forge project validate
stellar forge project info
```

### 3. Start the local loop

```bash
stellar forge dev up
stellar forge dev reseed --network local
stellar forge project sync
stellar forge project smoke
```

What that gives you:

- local RPC and Horizon endpoints exported into `.env.generated`
- regenerated project files after manifest edits
- a repeatable local sandbox for tokens, contracts, and generated apps

### 4. Build and preview a release

```bash
stellar forge contract build
stellar forge --dry-run release plan testnet
```

### 5. Deploy when you are ready

```bash
stellar forge release deploy testnet
stellar forge release verify testnet
stellar forge release env export testnet
```

For the full release model, including drift, history, rollback, and registry flows, use
[docs/deployment-guide.md](docs/deployment-guide.md).

## Generated Workspace Layout

The generated workspace usually revolves around these paths:

| Path | Purpose |
| --- | --- |
| `contracts/` | contract workspaces |
| `packages/` | generated bindings |
| `apps/api` | generated API scaffold |
| `apps/web` | generated frontend scaffold |
| `workers/events` | event ingestion scripts and cursor snapshot |
| `dist/` | deploy snapshots, registry artifacts, fetched Wasm files, and reports |
| `scripts/doctor.mjs` | wrapper for `stellar-forge doctor` |
| `scripts/reseed.mjs` | wrapper for `stellar-forge dev reseed` |
| `scripts/release.mjs` | wrapper for release plan/deploy/verify/env export/alias sync |

## Generated Apps

If the template enables API and frontend scaffolds, a common loop is:

```bash
pnpm --dir apps/api install
pnpm --dir apps/api dev
pnpm --dir apps/web install
pnpm --dir apps/web dev
stellar forge project smoke
stellar forge project smoke --browser
```

Swap `pnpm` for `npm`, `yarn`, or `bun` if `project.package_manager` says otherwise.

The generated browser smoke runner is also available as split steps inside `apps/web`:

- `smoke:browser:build`
- `smoke:browser:install`
- `smoke:browser:run`

Those scripts are documented in more detail in
[docs/command-reference.md](docs/command-reference.md#project-smoke).

## How To Work With This Repository

Local quality gates:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked
cargo audit
```

Contribution and project-process docs:

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [SUPPORT.md](SUPPORT.md)
- [SECURITY.md](SECURITY.md)
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## Troubleshooting

### `stellar forge` does not appear in `stellar plugin ls`

Make sure the `stellar-forge` binary is on `PATH`. Until plugin discovery works, use
`stellar-forge ...` directly.

### Commands fail because `stellar` is missing

Install the official `stellar` CLI first, then rerun `stellar forge doctor deps`.

### `dev up` fails

`dev up` expects `[networks.local]` to exist with `kind = "local"`. It also needs Docker for the
local network workflow.

### `events backfill` fails immediately

That command needs the API scaffold and persisted storage. Run `stellar forge events ingest init`
first, and make sure `sqlite3` is installed locally.

### `release verify` warns that the network may have reset

Shared test environments can lose state. When ids in `stellarforge.lock.json` no longer resolve,
redeploy or reseed the target environment and export runtime values again.
