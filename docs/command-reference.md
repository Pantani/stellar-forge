# Command Reference

Examples use `stellar forge ...`. Replace them with `stellar-forge ...` if the plugin is not
visible through `stellar plugin ls`.

## How To Use This Reference

This document aims to be exact first and narrative second.

Conventions used below:

- angle brackets such as `<env>` or `<contract>` mean a required positional value
- square brackets such as `[--out <path>]` mean an optional flag or argument
- examples assume your current directory already contains `stellarforge.toml` unless stated otherwise
- `--dry-run` is your safest default whenever a command could write files, deploy contracts, or
  call into the Stellar CLI
- `--json` prints the structured report to stdout, while `--out` persists that report to a file for
  later inspection
- `--` matters on passthrough commands such as `contract call` and `wallet sep7 contract-call`,
  because everything after it is forwarded to the underlying Stellar command

Resource names in examples usually refer to manifest keys:

- `rewards` means a contract declared in `[contracts.rewards]`
- `points` means a token declared in `[tokens.points]`
- `alice` means a wallet or identity declared in the manifest, depending on the command

Good default habits:

```bash
stellar forge --dry-run project validate
stellar forge --dry-run release plan testnet
stellar forge --json doctor
stellar forge --json release status testnet --out dist/release.status.json
```

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

Common report-writing pattern:

```bash
stellar forge --json project validate --out dist/project.validate.json
stellar forge --json release plan testnet --out dist/release.plan.json
stellar forge events export --path dist/events.json --out dist/events.export.json
```

For almost every command, `--out` writes the command report. The main exception is
`contract fetch`, where `--out` chooses the fetched Wasm artifact path itself.

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
stellar forge project info --out dist/project.info.json
stellar forge --json project info
```

### `project sync`

Regenerates derived files from the manifest, including `.env.example`, API files, frontend state,
and OpenAPI output when those modules are enabled.

```bash
stellar forge project sync
stellar forge project sync --out dist/project.sync.json
```

### `project validate`

Strict validation wrapper over project diagnostics. In human mode it exits non-zero when errors are
found.

```bash
stellar forge project validate
stellar forge project validate --out dist/project.validate.json
stellar forge --json project validate
```

### `project add`

Adds a managed module to an existing workspace.

```bash
stellar forge project add contract escrow --template escrow
stellar forge project add api
stellar forge project add frontend --framework react-vite
stellar forge project add contract escrow --template escrow --out dist/project.add.contract.json
stellar forge project add api --out dist/project.add.api.json
stellar forge project add frontend --framework react-vite --out dist/project.add.frontend.json
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
stellar forge project adopt scaffold --out dist/project.adopt.json
```

### `project smoke`

Runs the generated smoke checks for the current workspace.

Syntax:

```bash
stellar forge project smoke [--install] [--browser] [--out <path>]
```

Use it when you want a repo-level validation step before a release, after regenerating the
workspace, or after changing generated frontend files.

Common forms:

```bash
stellar forge project smoke
stellar forge project smoke --install
stellar forge project smoke --browser
stellar forge project smoke --install --browser
```

Behavior:

- `project smoke` runs the UI smoke runner in `apps/web`
- `--install` adds a package-manager install step first, which is useful on fresh checkouts or
  when `node_modules` is not present yet
- `--browser` switches the runner from the lighter UI smoke to the browser smoke path
- `--install --browser` is the safest end-to-end form when you need to verify both dependency
  setup and browser execution in one command
- `--out` writes the JSON report to a file path in addition to stdout

When `--browser` is set, the command uses the generated browser smoke runner and follows the same
split workflow as the package scripts below.

### Generated browser smoke scripts

The generated frontend workspace exposes incremental browser smoke scripts under `apps/web`:

```bash
pnpm --dir apps/web smoke:browser:build
pnpm --dir apps/web smoke:browser:install
pnpm --dir apps/web smoke:browser:run
pnpm --dir apps/web smoke:browser
```

Use them like this:

- `smoke:browser:build` when you changed frontend source, generated UI code, or want a fresh build
  artifact before a browser run
- `smoke:browser:install` when the Chromium bundle is missing, the cache was cleared, or the pinned
  Playwright revision changed
- `smoke:browser:run` when the build already exists and you only want to rerun the browser check
- `smoke:browser` when you want the full flow in one pass

Cache behavior:

- the full `smoke:browser` script checks the Playwright cache first
- if the pinned Chromium bundle is already present, it skips reinstalling the browser
- if the cache is cold, `smoke:browser` performs the install step before running the browser test
- if port `4173` is already busy, rerun with `STELLAR_FORGE_BROWSER_SMOKE_PORT=<port>`

## `doctor`

Runs environment, dependency, project, and network diagnostics.

### Common forms

```bash
stellar forge doctor
stellar forge doctor --out dist/doctor.json
stellar forge doctor env
stellar forge doctor env --out dist/doctor.env.json
stellar forge doctor deps
stellar forge doctor deps --out dist/doctor.deps.json
stellar forge doctor fix
stellar forge doctor audit --out dist/doctor.audit.json
stellar forge doctor fix --out dist/doctor.fix.json
stellar forge doctor project
stellar forge doctor project --out dist/doctor.project.json
stellar forge doctor network local
stellar forge doctor network local --out dist/doctor.network.json
```

### What each subcommand checks

| Command | Focus |
| --- | --- |
| `doctor` | Full diagnostic sweep |
| `doctor env` | Active cwd, manifest path, network, identity, output mode; `--out` writes the JSON report to disk |
| `doctor deps` | Presence of `stellar`, Docker, Rust, Node, `pnpm`, `sqlite3`, registry tooling, plugin detection; `--out` writes the JSON report to disk |
| `doctor audit` | Full project audit; `--out` writes the report to disk |
| `doctor fix` | Regenerate managed files such as scripts, API/frontend artifacts, env exports, and release snapshots; `--out` writes the report to disk |
| `doctor project` | Manifest validity, generated files, lockfile, release-state drift, scaffold compatibility; `--out` writes the JSON report to disk |
| `doctor network <env>` | RPC/Horizon reachability and deployed-resource probes for a target environment; `--out` writes the JSON report to disk |

## `dev`

Controls local quickstart flows and reseeding.

### `dev up`

Starts the local Stellar quickstart through `stellar container start local` and writes a local
`.env.generated`.

```bash
stellar forge dev up
stellar forge dev up --out dist/dev.up.json
```

### `dev down`

Stops the local quickstart.

```bash
stellar forge dev down
stellar forge dev down --out dist/dev.down.json
```

### `dev status`

Runs the same kind of checks as `doctor network local`.

```bash
stellar forge dev status
stellar forge dev status --out dist/dev.status.json
```

### `dev reset`

Restarts the local quickstart.

```bash
stellar forge dev reset
stellar forge dev reset --out dist/dev.reset.json
```

### `dev reseed`

Rehydrates identities, tokens, contracts, event state, and env exports for the selected network.

```bash
stellar forge dev reseed
stellar forge dev reseed --network local
stellar forge dev reseed --network testnet
stellar forge dev reseed --out dist/dev.reseed.json
```

### `dev snapshot save|load`

Captures or restores the local project state used during development for the active network.

```bash
stellar forge dev snapshot save baseline
stellar forge dev snapshot save baseline --out dist/dev.snapshot.save.json
stellar forge dev snapshot load baseline
stellar forge dev snapshot load baseline --out dist/dev.snapshot.load.json
```

Notes:

- snapshots currently include `stellarforge.lock.json`, `.env.generated`,
  `dist/deploy.<env>.json`, and `workers/events/cursors.json`
- use `--path` when you want a custom artifact location instead of the default
  `dist/snapshots/dev.<env>.<name>.json`
- when a managed snapshot is overwritten, the previous file is archived under
  `dist/snapshots/history/dev.<env>.<name>.<timestamp>.json`
- `load` first tries the current managed snapshot and falls back to the newest archived snapshot
  only when the current file is missing
- that fallback does not apply when `--path` is explicit
- `load` restores local files only; it does not reseed or mutate on-chain state

### `dev fund <target>`

Funds a named wallet, identity, or direct address using friendbot or the local root account.

```bash
stellar forge dev fund alice
stellar forge dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
stellar forge --network testnet --dry-run dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/dev.fund.json
```

### `dev events`

Points to the resource-specific event tooling and can capture that guidance as a report.

```bash
stellar forge dev events
stellar forge dev events rewards
stellar forge dev events rewards --out dist/dev.events.json
```

### `dev watch`

Polls contract source trees, rebuilds changed contracts, and refreshes generated API/frontend
files.

```bash
stellar forge dev watch
stellar forge dev watch --once
stellar forge dev watch --interval-ms 3000
stellar forge --dry-run --network local dev watch --once --out dist/dev.watch.json
```

Notes:

- `--json` requires `--once`
- this is file watching for managed contracts, not on-chain event streaming

### `dev logs`

Streams local quickstart logs.

```bash
stellar forge dev logs
stellar forge --dry-run dev logs --out dist/dev.logs.json
```

## `scenario`

Runs or previews manifest-declared scenario flows.

### `scenario run`

Executes the named scenario in manifest order and stops on the first failing step.

```bash
stellar forge scenario run checkout
stellar forge scenario run checkout --out dist/scenario.run.json
```

### `scenario test`

Validates the same scenario in preview mode by forcing the internal steps into dry-run behavior
and then evaluating any declared scenario assertions.

```bash
stellar forge scenario test checkout
stellar forge scenario test checkout --out dist/scenario.test.json
```

Notes:

- scenarios live under `[scenarios.<name>]` in `stellarforge.toml`
- step execution reuses the existing command implementations, so reports include the underlying
  `commands`, `warnings`, `artifacts`, and step-by-step status
- `scenario test` also evaluates typed assertions declared under `[[scenarios.<name>.assertions]]`
  and promotes failed assertions into normal CLI checks
- command-line `--network` and `--identity` still override the scenario defaults when present

## `contract`

Build, format, lint, deploy, inspect, fetch, bind, invoke, and manage TTL for contracts.

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
stellar forge contract build rewards --out dist/contract.build.json
```

### `contract format`

Formats one contract or all declared contracts with `cargo fmt`. Add `--check` when you only want
to verify formatting.

```bash
stellar forge contract format
stellar forge contract format rewards
stellar forge contract format rewards --check
stellar forge contract format rewards --check --out dist/contract.format.json
```

### `contract lint`

Runs `cargo clippy --all-targets --all-features -- -D warnings` for one contract or all declared
contracts.

```bash
stellar forge contract lint
stellar forge contract lint rewards
stellar forge contract lint rewards --out dist/contract.lint.json
```

### `contract deploy`

Deploys a single declared contract and updates the lockfile. Use `--out` to write the JSON report
to disk.

```bash
stellar forge contract deploy rewards
stellar forge contract deploy rewards --env testnet
stellar forge contract deploy rewards --out dist/contract.deploy.json
```

### `contract call`

Syntax:

```bash
stellar forge contract call <contract> <function> [--send <mode>] [--build-only] [--out <path>] [-- <args...>]
```

Important detail: arguments after the function are passed through to `stellar contract invoke`, so
the `--` separator matters.

Examples:

```bash
stellar forge contract call rewards award_points -- --member alice --amount 100
stellar forge contract call rewards spend_points --send no -- --member alice --amount 50
stellar forge contract call rewards award_points --build-only -- --member alice --amount 25
stellar forge contract call rewards award_points --out dist/contract.call.json -- --member alice --amount 25
```

### `contract bind`

Generates bindings under `packages/`. If `--lang` is omitted, the implementation defaults to
TypeScript.

```bash
stellar forge contract bind rewards --lang typescript,python
stellar forge contract bind rewards --lang typescript --out dist/contract.bind.json
```

### `contract info`

Summarizes manifest, deployment, bindings, and contract metadata.

```bash
stellar forge contract info rewards
stellar forge contract info rewards --out dist/contract.info.json
```

### `contract fetch`

Fetches Wasm from the network into `dist/contracts/<name>.<env>.wasm` unless `--out` is supplied.
Unlike the report-oriented commands, `--out` here controls the fetched Wasm artifact path itself.

```bash
stellar forge contract fetch rewards
stellar forge contract fetch rewards --out ./tmp/rewards.wasm
```

### `contract spec`

Prints contract info oriented around interface/spec inspection.

```bash
stellar forge contract spec rewards
stellar forge contract spec rewards --out dist/contract.spec.json
```

### `contract ttl extend|restore`

Manage TTL with the underlying Stellar CLI. Use `--out` to write the JSON report to disk.

```bash
stellar forge contract ttl extend rewards --ledgers 17280
stellar forge contract ttl restore rewards --key all
stellar forge contract ttl extend rewards --durability temporary --build-only
stellar forge contract ttl extend rewards --out dist/contract.ttl.extend.json
stellar forge contract ttl restore rewards --out dist/contract.ttl.restore.json
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
stellar forge token info points --out dist/token.info.json
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
stellar forge token balance points --holder alice --out dist/token.balance.json
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
stellar forge wallet ls --out dist/wallet.ls.json
```

### `wallet address`

```bash
stellar forge wallet address alice
stellar forge wallet address alice --out dist/wallet.address.json
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
stellar forge wallet balances alice --out dist/wallet.balances.json
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

### `wallet batch-pay`

Runs a list of payments from a JSON or CSV file.

JSON accepts either a top-level array or an object with `payments`, using fields `to`, `amount`,
and optional `asset`. CSV expects headers `to,amount,asset`.

```bash
stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json
stellar forge wallet batch-pay --from treasury --file payouts.csv --format csv
stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json --out dist/payouts.pay.json
```

### `wallet batch-validate`, `wallet batch-preview`, `wallet batch-summary`

Reads a batch file without sending payments. `batch-validate` checks that each row can be
interpreted, `batch-preview` returns the normalized entries, and `batch-summary` emits only the
aggregate counts.

```bash
stellar forge wallet batch-validate --from treasury --asset points --file payouts.json
stellar forge wallet batch-preview --from treasury --asset points --file payouts.csv --format csv
stellar forge wallet batch-summary --from treasury --asset points --file payouts.json
stellar forge wallet batch-validate --from treasury --asset points --file payouts.json --out dist/payouts.validate.json
stellar forge wallet batch-preview --from treasury --asset points --file payouts.csv --format csv --out dist/payouts.preview.json
stellar forge wallet batch-summary --from treasury --asset points --file payouts.json --out dist/payouts.summary.json
```

### `wallet batch-report`

Combines the normalized preview with the aggregate batch summary for review before sending.

```bash
stellar forge wallet batch-report --from treasury --asset points --file payouts.json
stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json --out dist/payouts.report.json
```

### `wallet batch-reconcile`

Compares a batch file against a previously captured execution report and highlights missing,
unexpected, or mismatched rows.

```bash
stellar forge wallet batch-reconcile --from treasury --asset points --file payouts.json --report dist/payouts.report.json
stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.reconcile.json
```

### `wallet batch-resume`

Resumes a batch-payment run from the same file inputs and a prior execution report, skipping rows
that were already captured.

```bash
stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json
stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.resume.json
```

### `wallet receive`

Print a wallet address, and optionally return SEP-7 and QR payload data.

```bash
stellar forge wallet receive alice
stellar forge wallet receive alice --sep7 --asset points
stellar forge wallet receive alice --qr --asset XLM
stellar forge wallet receive alice --sep7 --asset points --out dist/wallet.receive.json
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

Scaffolds smart-wallet onboarding helpers and metadata.

```bash
stellar forge wallet smart create team-safe --mode ed25519
stellar forge wallet smart create checkout-passkey --mode passkey
stellar forge wallet smart onboard checkout-passkey
stellar forge wallet smart materialize checkout-passkey
stellar forge wallet smart controller rotate checkout-passkey alice
stellar forge wallet smart scaffold checkout-passkey
stellar forge wallet smart info checkout-passkey
stellar forge wallet smart info checkout-passkey --out dist/wallet.smart.info.json
```

### `wallet smart onboard`

Summarizes the onboarding app, policy contract, environment values, and next steps for a smart
wallet.

```bash
stellar forge wallet smart onboard checkout-passkey
stellar forge wallet smart onboard checkout-passkey --out dist/wallet.smart.onboard.json
```

Notes:

- use it when you want the browser-facing onboarding app and policy contract in place for a
  smart wallet
- the command keeps the generated onboarding app path and policy metadata aligned with the wallet
  entry

### `wallet smart controller rotate`

Updates the controller identity for a smart wallet.

```bash
stellar forge wallet smart controller rotate checkout-passkey alice
stellar forge wallet smart controller rotate checkout-passkey alice --out dist/wallet.smart.controller.rotate.json
```

Notes:

- use it when the wallet should point at a new controller identity
- the command keeps the wallet metadata and generated app references aligned with the new
  controller

### `wallet smart materialize`

Materializes the smart wallet into workspace state.

```bash
stellar forge wallet smart materialize checkout-passkey
stellar forge wallet smart materialize checkout-passkey --out dist/wallet.smart.materialize.json
```

Notes:

- use it when you want the smart wallet to resolve to concrete generated paths and deployment
  metadata
- it pairs well with `wallet smart info` when you want to inspect the materialized state

### `wallet smart policy`

Operates the generated policy contract attached to a smart wallet.

```bash
stellar forge wallet smart policy info guardian
stellar forge wallet smart policy info guardian --out dist/wallet.smart.policy.info.json
stellar forge wallet smart policy simulate guardian --file policy.toml
stellar forge wallet smart policy simulate guardian --file policy.json
stellar forge wallet smart policy set-daily-limit guardian 1250
stellar forge wallet smart policy allow guardian GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
stellar forge wallet smart policy revoke guardian GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --build-only
```

Notes:

- these commands work against the policy contract declared on the smart wallet
- `wallet smart policy simulate` reuses the same policy-file format as `apply`, but always forces
  preview mode so you can inspect the planned mutations before sending them
- policy files can be written as `.toml` or `.json`
- if the policy contract is not deployed yet, dry runs can still preview the invoke command
- the source account defaults to the controller identity when one exists, otherwise the active identity

### `wallet smart provision`

Records the smart-wallet contract id locally and refreshes the generated onboarding scaffold.
Use `--address` when the contract id already exists, and `--fund` when the controller identity
should be topped up while you align the local workspace.

```bash
stellar forge wallet smart provision checkout-passkey
stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
stellar forge wallet smart provision checkout-passkey --fund
stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/wallet.smart.provision.json
```

Notes:

- use it after the smart-wallet contract account already exists on-chain
- the command keeps `stellarforge.toml` and the onboarding app aligned with the chosen contract id

### `wallet smart policy sync`

Refreshes local smart-wallet policy metadata from the deployed policy contract.

```bash
stellar forge wallet smart policy sync checkout-passkey
stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json
```

Notes:

- use it when the deployed policy admin may no longer match the local controller identity
- if the on-chain admin resolves to a declared local identity, the scaffold is refreshed to match

### `wallet smart policy diff`

Compares local smart-wallet policy metadata against the deployed policy contract and reports drift.

```bash
stellar forge wallet smart policy diff checkout-passkey
stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json
```

Notes:

- use it to compare the local controller identity against the observed on-chain policy admin
- the report also includes the observed daily limit when live policy probes are available

## `token`

Create and operate assets, SAC wrappers, contract tokens, and batch distributions.

### `token airdrop`

Applies a batch payout file to a specific token. The input format matches `wallet batch-pay`, but
the token name supplies the asset automatically.

```bash
stellar forge token airdrop points --from treasury --file rewards.csv --format csv
stellar forge token airdrop points --file rewards.json
stellar forge token airdrop points --from treasury --file rewards.csv --format csv --out dist/airdrop.json
```

### `token airdrop-validate`, `token airdrop-preview`, `token airdrop-summary`

Token-scoped wrappers around the wallet batch-file helpers. The token name supplies the asset for
every row automatically.

```bash
stellar forge token airdrop-validate points --file rewards.csv --format csv
stellar forge token airdrop-preview points --from treasury --file rewards.json
stellar forge token airdrop-summary points --file rewards.csv --format csv
stellar forge token airdrop-validate points --file rewards.csv --format csv --out dist/airdrop.validate.json
stellar forge token airdrop-preview points --from treasury --file rewards.json --out dist/airdrop.preview.json
stellar forge token airdrop-summary points --file rewards.csv --format csv --out dist/airdrop.summary.json
```

### `token airdrop-report`

Builds the same review-oriented report as `wallet batch-report`, but pins every row to the named
token automatically.

```bash
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv --out dist/airdrop.report.json
```

### `token airdrop-reconcile`

Compares a token airdrop batch against a previously captured report and highlights missing,
unexpected, or mismatched rows while preserving token context.

```bash
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.reconcile.json
```

### `token airdrop-resume`

Resumes a token airdrop from the same batch-file inputs and prior report flow used by the wallet
batch commands.

```bash
stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json
stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.resume.json
```

## `api`

Generate or refresh the managed API scaffold.

### `api init`

Creates `apps/api` and turns API support on in the manifest.

```bash
stellar forge api init
stellar forge api init --out dist/api.init.json
```

### `api generate contract|token`

Generates a resource service around a specific contract or token.

```bash
stellar forge api generate contract rewards
stellar forge api generate token points
stellar forge api generate contract rewards --out dist/api.generate.contract.json
stellar forge api generate token points --out dist/api.generate.token.json
```

### `api openapi export`

Regenerates `apps/api/openapi.json`.

```bash
stellar forge api openapi export
stellar forge api openapi export --out dist/api.openapi.json
```

### `api events init`

Ensures event-ingestion support exists in the API scaffold.

```bash
stellar forge api events init
stellar forge api events init --out dist/api.events.init.json
```

### `api relayer init`

Adds relayer endpoints and support files to the API scaffold.

```bash
stellar forge api relayer init
stellar forge api relayer init --out dist/api.relayer.init.json
```

## `events`

Stream contract/account events, seed recent event history, and manage cursors.
The CLI also exposes a status report for the local event store.

### `events watch`

Syntax:

```bash
stellar forge events watch <kind> <resource> [--out <path>]
```

Supported kinds are `contract`, `token`, and `account`.

Examples:

```bash
stellar forge events watch contract rewards
stellar forge events watch token points
stellar forge events watch account alice
stellar forge events watch contract rewards --topic sym:PointsAwarded
stellar forge events watch contract rewards --count 100 --start-ledger 123456
stellar forge events watch contract rewards --out dist/events.watch.json
```

### `events ingest init`

Bootstraps event-ingestion support. Internally it aligns with API event scaffold generation.

```bash
stellar forge events ingest init
stellar forge events ingest init --out dist/events.ingest.init.json
```

### `events status`

Summarizes the local event store, including total events, latest ledger information, and cursor
counts. Use `--out` to write the JSON report to disk.

```bash
stellar forge events status
stellar forge events status --out dist/events.status.json
```

### `events cursor ls`

Lists persisted event cursors from sqlite or the JSON snapshot.

```bash
stellar forge events cursor ls
stellar forge events cursor ls --out dist/events.cursor.json
```

### `events cursor reset`

```bash
stellar forge events cursor reset testnet:contract:rewards
stellar forge events cursor reset testnet:contract:rewards --out dist/events.cursor.reset.json
```

### `events backfill`

Backfills recent history into the local sqlite event store. The command is retention-bound by the
provider behind the RPC/Horizon endpoints. Use `--out` to write the JSON report to disk.

Accepted resource forms:

- contract name, for example `rewards`
- token name, for example `points`
- explicit prefix, for example `contract:rewards`, `token:points`, `account:alice`

Examples:

```bash
stellar forge events backfill contract:rewards --count 200
stellar forge events backfill points --start-ledger 123456
stellar forge events backfill account:alice --count 100
stellar forge events backfill contract:rewards --count 200 --out dist/events.backfill.json
```

Notes:

- this command requires the API scaffold and `sqlite3`
- public RPC history is short-lived; use backfill to seed recent history, not as a permanent archive
- topic filters support comma-separated segments and wildcards such as `COUNTER,*` or `sym:Transfer,*`

## `release`

Plan, deploy, inspect, diff, prune, export env, sync aliases, and run registry workflows.

### `release plan <env>`

Shows the commands that would run, the identities required, expected lockfile changes, and which
artifacts will be produced. Use `--out` to write the JSON report to disk.

```bash
stellar forge release plan testnet
stellar forge --dry-run release plan futurenet
stellar forge release plan testnet --out dist/release.plan.json
```

### `release deploy <env>`

Performs the release for a target environment. Use `--out` to write the JSON report to disk.

```bash
stellar forge release deploy testnet
stellar forge release deploy pubnet --confirm-mainnet
stellar forge release deploy testnet --out dist/release.deploy.json
```

Important behavior:

- if `[release.<env>]` exists, only its listed contracts and tokens are deployed
- otherwise all declared contracts and tokens are considered part of the release
- `pubnet` is guarded by `--confirm-mainnet`

### `release verify <env>`

Validates deploy artifacts, lockfile consistency, event worker config, and optionally probes
deployed contract IDs on-chain. Use `--out` to write the JSON report to disk.

```bash
stellar forge release verify testnet
stellar forge release verify testnet --out dist/release.verify.json
```

### `release status <env>`

Collects the current release checks together with the current snapshot and the newest archived
snapshot, when present. Use `--out` to write the JSON report to disk.

```bash
stellar forge release status testnet
stellar forge release status testnet --out dist/release.status.json
```

### `release diff <env>`

Compares the current release snapshot against a selected artifact. Without `--path`, it uses the
newest archived artifact when one exists. Use `--out` to write the JSON report to disk.

```bash
stellar forge release diff testnet
stellar forge release diff testnet --path dist/history/deploy.testnet.20260413T000000.000Z.json
stellar forge release diff testnet --out dist/release.diff.json
```

### `release drift <env>`

Compares the release state for an environment against the current workspace and reports drift in
manifests, lockfile data, and generated artifacts. Use `--out` to write the JSON report to disk.

```bash
stellar forge release drift testnet
stellar forge release drift testnet --out dist/release.drift.json
```

### `release history <env>`

Lists the current deploy snapshot and archived snapshots under `dist/history/`. Use `--out` to
write the JSON report to disk.

```bash
stellar forge release history testnet
stellar forge release history testnet --out dist/release.history.json
```

### `release inspect <env>`

Shows a summary for the current deploy artifact, or for a specific historical artifact when `--path`
is provided. Use `--out` to write the JSON report to disk.

```bash
stellar forge release inspect testnet
stellar forge release inspect testnet --path dist/history/deploy.testnet.20260413T000000.000Z.json
stellar forge release inspect testnet --out dist/release.inspect.json
```

### `release rollback <env>`

Restores local release metadata for an environment from a previous deploy snapshot. By default it
uses the newest file under `dist/history/`; `--to` can point at a specific artifact. Use `--out`
to write the JSON report to disk.

```bash
stellar forge release rollback testnet
stellar forge release rollback testnet --to dist/history/deploy.testnet.20260413T000000.000Z.json
stellar forge release rollback testnet --out dist/release.rollback.json
```

Notes:

- this restores `stellarforge.lock.json`, `.env.generated`, and `dist/deploy.<env>.json`
- it does not revert on-chain contracts or token state

### `release prune <env>`

Deletes old archived release artifacts under `dist/history/`, keeping only the newest `N`.

```bash
stellar forge release prune testnet
stellar forge release prune testnet --keep 3
stellar forge release prune testnet --keep 3 --out dist/release.prune.json
```

### `release aliases sync <env>`

Synchronizes Stellar CLI aliases from manifest and lockfile state. Use `--out` to write the JSON
report to disk.

```bash
stellar forge release aliases sync testnet
stellar forge release aliases sync testnet --out dist/release.aliases.json
```

### `release env export <env>`

Writes `.env.generated` and the deploy snapshot for the target environment. Use `--out` to write
the JSON report to disk.

```bash
stellar forge release env export testnet
stellar forge release env export testnet --out dist/release.env.json
```

### `release registry publish <contract>`

Publishes registry metadata for a contract using the active network from `--network` or
`defaults.network`.

```bash
stellar forge --network testnet release registry publish rewards
stellar forge --network testnet release registry publish rewards --out dist/release.registry.publish.json
```

### `release registry deploy <contract>`

Deploys a contract using registry metadata and updates `dist/registry.<env>.json`.

```bash
stellar forge --network testnet release registry deploy rewards
stellar forge --network testnet release registry deploy rewards --out dist/release.registry.deploy.json
```

Registry notes:

- the command first tries `stellar registry ...`
- if that subcommand is unavailable, it can fall back to the standalone `stellar-registry` binary
- set `STELLAR_FORGE_REGISTRY_MODE=stellar` or `STELLAR_FORGE_REGISTRY_MODE=dedicated` for a
  deterministic backend choice

## Additional command surfaces

These command surfaces are available in the CLI and are useful as standalone entry points.

### `scenario run|test --out`

Writes scenario execution or preview reports to disk.

```bash
stellar forge scenario run checkout --out dist/scenario.run.json
stellar forge scenario test checkout --out dist/scenario.test.json
```

### `wallet smart policy apply`

Applies a policy file to a named smart wallet and keeps the local scaffold aligned with that file.

```bash
stellar forge wallet smart policy apply checkout-passkey --file policy.toml
stellar forge wallet smart policy apply checkout-passkey --file policy.toml --out dist/wallet.smart.policy.apply.json
```

### `wallet smart policy simulate`

Previews a policy file against a named smart wallet without sending any on-chain mutations.

```bash
stellar forge wallet smart policy simulate checkout-passkey --file policy.toml
stellar forge wallet smart policy simulate checkout-passkey --file policy.json
stellar forge wallet smart policy simulate checkout-passkey --file policy.toml --out dist/wallet.smart.policy.simulate.json
```

### `contract new --out`

Writes the scaffold summary for a new contract template to disk.

```bash
stellar forge contract new escrow --template escrow --out dist/contract.new.json
```

### `token create --out`

Writes the token-creation plan to disk for classic or contract token setups.

```bash
stellar forge --dry-run --network testnet token create credits --mode contract --metadata-name "Store Credit" --initial-supply 25 --out dist/token.create.json
```

### `token mint|burn|transfer --out`

Writes token movement and supply mutation reports to disk.

```bash
stellar forge --dry-run --network testnet token mint credits --to alice --amount 10 --from issuer --out dist/token.mint.json
stellar forge token burn points --amount 5 --from treasury --out dist/token.burn.json
stellar forge token transfer points --to alice --amount 10 --from treasury --out dist/token.transfer.json
```

### `token trust|freeze|unfreeze|clawback --out`

Writes classic token trustline and authorization mutations to disk.

```bash
stellar forge token trust points alice --out dist/token.trust.json
stellar forge token freeze points alice --out dist/token.freeze.json
stellar forge token unfreeze points alice --out dist/token.unfreeze.json
stellar forge token clawback points alice 1 --out dist/token.clawback.json
```

### `token sac id|deploy --out`

Writes SAC discovery and deployment reports to disk.

```bash
stellar forge --network testnet token sac id points --out dist/token.sac.id.json
stellar forge --network testnet token sac deploy points --out dist/token.sac.deploy.json
```

### `token contract init --out`

Writes the contract-token initialization report to disk.

```bash
stellar forge --network testnet token contract init credits --out dist/token.contract.init.json
```

### `project add contract --out`

Adds a contract module and writes the project-add report to disk.

```bash
stellar forge project add contract escrow --template escrow --out dist/project.add.contract.json
```

### `project adopt scaffold --out`

Imports a Scaffold-style workspace and writes the adoption report to disk.

```bash
stellar forge project adopt scaffold --out dist/project.adopt.json
```

### `project add api --out`

Adds API support and writes the project-add report to disk.

```bash
stellar forge project add api --out dist/project.add.api.json
```

### `project add frontend --out`

Adds a frontend scaffold and writes the project-add report to disk.

```bash
stellar forge project add frontend --framework react-vite --out dist/project.add.frontend.json
```

### `api init --out`

Creates or refreshes the API scaffold and writes the report to disk.

```bash
stellar forge api init --out dist/api.init.json
```

### `api generate contract --out`

Generates a contract service and writes the report to disk.

```bash
stellar forge api generate contract rewards --out dist/api.generate.contract.json
```

### `api generate token --out`

Generates a token service and writes the report to disk.

```bash
stellar forge api generate token points --out dist/api.generate.token.json
```

### `api openapi export --out`

Exports OpenAPI output and writes the report to disk.

```bash
stellar forge api openapi export --out dist/api.openapi.json
```

### `api events init --out`

Adds event-ingestion support and writes the report to disk.

```bash
stellar forge api events init --out dist/api.events.init.json
```

### `api relayer init --out`

Adds relayer support and writes the report to disk.

```bash
stellar forge api relayer init --out dist/api.relayer.init.json
```

### `project info --out`

Writes the project info report to disk.

```bash
stellar forge project info --out dist/project.info.json
```

### `project validate --out`

Writes the project validation report to disk.

```bash
stellar forge project validate --out dist/project.validate.json
```

### `contract build --out`

Writes the contract-build report to disk.

```bash
stellar forge contract build rewards --out dist/contract.build.json
```

### `contract format --out`

Writes the contract-format report to disk.

```bash
stellar forge contract format rewards --check --out dist/contract.format.json
```

### `contract lint --out`

Writes the contract-lint report to disk.

```bash
stellar forge contract lint rewards --out dist/contract.lint.json
```

### `contract deploy --out`

Writes the contract-deploy report to disk.

```bash
stellar forge contract deploy rewards --out dist/contract.deploy.json
```

### `contract call --out`

Writes the contract-call report to disk.

```bash
stellar forge contract call rewards award_points --out dist/contract.call.json -- --member alice --amount 25
```

### `contract bind --out`

Writes the contract-bind report to disk.

```bash
stellar forge contract bind rewards --lang typescript --out dist/contract.bind.json
```

### `contract spec --out`

Writes the contract spec-oriented report to disk.

```bash
stellar forge contract spec rewards --out dist/contract.spec.json
```

### `contract ttl extend|restore --out`

Writes the contract TTL mutation report to disk.

```bash
stellar forge contract ttl extend rewards --out dist/contract.ttl.extend.json
stellar forge contract ttl restore rewards --out dist/contract.ttl.restore.json
```

### `token balance --out`

Writes the filtered token-balance report to disk.

```bash
stellar forge token balance points --holder alice --out dist/token.balance.json
```

### `wallet ls --out`

Writes the wallet inventory report to disk.

```bash
stellar forge wallet ls --out dist/wallet.ls.json
```

### `wallet address --out`

Writes the resolved wallet address report to disk.

```bash
stellar forge wallet address alice --out dist/wallet.address.json
```

### `wallet balances --out`

Writes the wallet balances report to disk.

```bash
stellar forge wallet balances alice --out dist/wallet.balances.json
```

### `wallet receive --out`

Writes the wallet receive payload report to disk.

```bash
stellar forge wallet receive alice --sep7 --asset points --out dist/wallet.receive.json
```

### `wallet create --out`

Writes the wallet-creation report to disk.

```bash
stellar forge wallet create bob --fund --out dist/wallet.create.json
```

### `wallet fund --out`

Writes the friendbot or funding report to disk.

```bash
stellar forge wallet fund alice --out dist/wallet.fund.json
```

### `wallet trust --out`

Writes the trustline creation report to disk.

```bash
stellar forge wallet trust alice points --out dist/wallet.trust.json
```

### `wallet pay --out`

Writes the payment report to disk.

```bash
stellar forge wallet pay --from treasury --to alice --asset points --amount 10 --out dist/wallet.pay.json
```

### `wallet sep7 payment --out`

Writes the SEP-7 payment handoff report to disk.

```bash
stellar forge wallet sep7 payment --from treasury --to alice --asset points --amount 10 --out dist/wallet.sep7.payment.json
```

### `wallet sep7 contract-call --out`

Writes the SEP-7 contract-call handoff report to disk.

```bash
stellar forge --network testnet wallet sep7 contract-call rewards award_points --out dist/wallet.sep7.contract-call.json -- --member alice --amount 25
```

### `project sync --out`

Writes the project sync report to disk.

```bash
stellar forge project sync --out dist/project.sync.json
```

### `wallet smart create --out`

Writes the smart-wallet creation report to disk.

```bash
stellar forge wallet smart create sentinel --mode ed25519 --out dist/wallet.smart.create.json
```

### `wallet smart scaffold --out`

Writes the smart-wallet scaffold summary to disk.

```bash
stellar forge wallet smart scaffold guardian --out dist/wallet.smart.scaffold.json
```

### `wallet smart onboard --out`

Writes the smart-wallet onboarding summary to disk.

```bash
stellar forge wallet smart onboard checkout-passkey --out dist/wallet.smart.onboard.json
```

### `wallet smart policy info --out`

Writes the smart-wallet policy info report to disk.

```bash
stellar forge wallet smart policy info guardian --out dist/wallet.smart.policy.info.json
```

### `wallet smart policy set-daily-limit|allow|revoke --out`

Writes smart-wallet policy mutation reports to disk.

```bash
stellar forge wallet smart policy set-daily-limit sentinel 1250 --build-only --out dist/wallet.smart.policy.set-daily-limit.json
stellar forge wallet smart policy allow sentinel alice --build-only --out dist/wallet.smart.policy.allow.json
stellar forge wallet smart policy revoke sentinel alice --build-only --out dist/wallet.smart.policy.revoke.json
```

### `wallet smart policy sync --out`

Writes the smart-wallet policy sync report to disk.

```bash
stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json
```

### `wallet smart policy diff --out`

Writes the smart-wallet policy diff report to disk.

```bash
stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json
```

### `dev snapshot save|load --out`

Writes development snapshot save/load reports to disk.

```bash
stellar forge dev snapshot save baseline --out dist/dev.snapshot.save.json
stellar forge dev snapshot load baseline --out dist/dev.snapshot.load.json
```

### `dev status --out`

Writes the local dev status report to disk.

```bash
stellar forge dev status --out dist/dev.status.json
```

### `dev up --out`

Writes the local quickstart start report to disk.

```bash
stellar forge dev up --out dist/dev.up.json
```

### `dev down --out`

Writes the local quickstart stop report to disk.

```bash
stellar forge dev down --out dist/dev.down.json
```

### `dev reset --out`

Writes the local quickstart reset report to disk.

```bash
stellar forge dev reset --out dist/dev.reset.json
```

### `dev reseed --out`

Writes the reseed plan/report to disk.

```bash
stellar forge dev reseed --out dist/dev.reseed.json
```

### `dev fund <target> --out`

Writes the funding request report to disk.

```bash
stellar forge --network testnet --dry-run dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/dev.fund.json
```

### `dev watch --out`

Writes the one-shot watch bootstrap report to disk.

```bash
stellar forge --dry-run --network local dev watch --once --out dist/dev.watch.json
```

### `dev events --out`

Writes the event-tooling guidance report to disk.

```bash
stellar forge dev events rewards --out dist/dev.events.json
```

### `dev logs --out`

Writes the dry-run log-stream plan to disk.

```bash
stellar forge --dry-run dev logs --out dist/dev.logs.json
```

### `contract info --out`

Writes the contract info report to disk.

```bash
stellar forge contract info rewards --out dist/contract.info.json
```

### `token info --out`

Writes the token info report to disk.

```bash
stellar forge token info points --out dist/token.info.json
```

### `wallet smart info --out`

Writes the smart-wallet info report to disk.

```bash
stellar forge wallet smart info checkout-passkey --out dist/wallet.smart.info.json
```

### `release plan <env>`

Shows the commands that would run, the identities required, expected lockfile changes, and which
artifacts will be produced. Use `--out` to write the JSON report to disk.

```bash
stellar forge release plan testnet
stellar forge release plan testnet --out dist/release.plan.json
```

### `release deploy <env> --out`

Writes the release-deploy report to disk.

```bash
stellar forge release deploy testnet --out dist/release.deploy.json
```

### `release verify <env>`

Validates deploy artifacts, lockfile consistency, event worker config, and optionally probes
deployed contract IDs on-chain. Use `--out` to write the JSON report to disk.

```bash
stellar forge release verify testnet
stellar forge release verify testnet --out dist/release.verify.json
```

### `release aliases sync <env>`

Synchronizes Stellar CLI aliases from manifest and lockfile state. Use `--out` to write the JSON
report to disk.

```bash
stellar forge release aliases sync testnet
stellar forge release aliases sync testnet --out dist/release.aliases.json
```

### `release env export <env>`

Writes `.env.generated` and the deploy snapshot for the target environment. Use `--out` to write
the JSON report to disk.

```bash
stellar forge release env export testnet
stellar forge release env export testnet --out dist/release.env.json
```

### `release registry publish <contract> --out`

Writes the registry-publish report to disk.

```bash
stellar forge --network testnet release registry publish rewards --out dist/release.registry.publish.json
```

### `release registry deploy <contract> --out`

Writes the registry-deploy report to disk.

```bash
stellar forge --network testnet release registry deploy rewards --out dist/release.registry.deploy.json
```

### `wallet batch-pay --out`

Writes the batch-pay execution plan and normalized preview to disk.

```bash
stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json --out dist/payouts.pay.json
```

### `wallet batch-validate|batch-preview|batch-summary --out`

Writes the non-sending batch analysis report to disk.

```bash
stellar forge wallet batch-validate --from treasury --asset points --file payouts.json --out dist/payouts.validate.json
stellar forge wallet batch-preview --from treasury --asset points --file payouts.csv --format csv --out dist/payouts.preview.json
stellar forge wallet batch-summary --from treasury --asset points --file payouts.json --out dist/payouts.summary.json
```

### `wallet batch-report --out`

Writes generated batch-report style output to disk.

```bash
stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json --out dist/payouts.report.json
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv --out dist/airdrop.report.json
```

### `wallet batch-reconcile --out`

Writes the batch reconciliation report to disk.

```bash
stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.reconcile.json
```

### `wallet batch-resume --out`

Writes the resumed batch-payment plan to disk.

```bash
stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.resume.json
```

### `token airdrop --out`

Writes the token-airdrop execution plan and normalized preview to disk.

```bash
stellar forge token airdrop points --from treasury --file rewards.csv --format csv --out dist/airdrop.json
```

### `token airdrop-validate|airdrop-preview|airdrop-summary --out`

Writes the token-airdrop analysis report to disk.

```bash
stellar forge token airdrop-validate points --file rewards.csv --format csv --out dist/airdrop.validate.json
stellar forge token airdrop-preview points --from treasury --file rewards.json --out dist/airdrop.preview.json
stellar forge token airdrop-summary points --file rewards.csv --format csv --out dist/airdrop.summary.json
```

### `token airdrop-report --out`

Writes the token-airdrop report output to disk.

```bash
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv --out dist/airdrop.report.json
```

### `release prune <env> --out`

Writes the release-prune report to disk.

```bash
stellar forge release prune testnet --keep 3 --out dist/release.prune.json
```

### `token airdrop-reconcile --out`

Writes the token-airdrop reconciliation report to disk.

```bash
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.reconcile.json
```

### `token airdrop-resume --out`

Writes the resumed token-airdrop plan to disk.

```bash
stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.resume.json
```

### `events export`

Exports the local event-store snapshot for archival or inspection.

```bash
stellar forge events export
stellar forge events export --path dist/events.json
stellar forge events export --path dist/events.json --out dist/events.export.json
```

### `events watch --out`

Writes the live events-watch plan/report to disk.

```bash
stellar forge events watch contract rewards --out dist/events.watch.json
```

### `events ingest init --out`

Writes the events-ingest scaffold report to disk.

```bash
stellar forge events ingest init --out dist/events.ingest.init.json
```

### `events status --out`

Writes the event-store summary report to disk.

```bash
stellar forge events status --out dist/events.status.json
```

### `events cursor ls --out`

Writes the persisted event cursor snapshot to disk.

```bash
stellar forge events cursor ls --out dist/events.cursor.json
```

### `events cursor reset --out`

Writes the cursor-reset report to disk.

```bash
stellar forge events cursor reset testnet:contract:rewards --out dist/events.cursor.reset.json
```

### `events replay`

Replays a previously exported event-store snapshot back into the local sqlite store and cursor
snapshot. When `--path` is omitted, it falls back to the active network's default export path.

```bash
stellar forge events replay
stellar forge events replay --path dist/events.json
stellar forge events replay --path dist/events.json --out dist/events.replay.json
```

### `doctor audit`

Runs the higher-signal project audit pass over the workspace and managed outputs. Use `--out` to
write the JSON report to disk.

```bash
stellar forge doctor audit
stellar forge doctor audit --out dist/doctor.audit.json
```

### `doctor --out`

Runs the full diagnostic sweep and writes the JSON report to disk.

```bash
stellar forge doctor --out dist/doctor.json
```

### `doctor env`

Reports the active cwd, manifest path, network, identity, and output mode. Use `--out` to write
the JSON report to disk.

```bash
stellar forge doctor env
stellar forge doctor env --out dist/doctor.env.json
```

### `doctor deps`

Checks for `stellar`, Docker, Rust, Node, `pnpm`, `sqlite3`, registry tooling, and plugin
detection. Use `--out` to write the JSON report to disk.

```bash
stellar forge doctor deps
stellar forge doctor deps --out dist/doctor.deps.json
```

### `doctor project`

Checks manifest validity, generated files, lockfile, release-state drift, and scaffold
compatibility. Use `--out` to write the JSON report to disk.

```bash
stellar forge doctor project
stellar forge doctor project --out dist/doctor.project.json
```

### `doctor network <env>`

Checks RPC/Horizon reachability and deployed-resource probes for a target environment. Use `--out`
to write the JSON report to disk.

```bash
stellar forge doctor network local
stellar forge doctor network local --out dist/doctor.network.json
```

### `doctor fix --scope`

Narrows the repair pass to a managed area such as `scripts`, `events`, `api`, `frontend`,
`release`, or `lockfile`. The selective scopes are also available one at a time when you want a
smaller repair pass. Use `--out` to write the JSON report to disk.

```bash
stellar forge doctor fix --scope events
stellar forge doctor fix --scope api
stellar forge doctor fix --scope release
stellar forge doctor fix --scope release --out dist/doctor.fix.json
```

## Generated Helper Scripts

Fresh projects include small Node wrappers under `scripts/`:

| Script | What it does |
| --- | --- |
| `node scripts/doctor.mjs` | Runs `stellar-forge doctor` using `.env.generated` / `.env.example` defaults |
| `node scripts/reseed.mjs` | Runs `stellar-forge dev reseed` using env defaults |
| `node scripts/release.mjs --plan` | Wraps release plan/deploy/verify/env-export/aliases-sync |
| `node workers/events/ingest-events.mjs <resource> --status --once` | Event ingest loop for tracked resources via `events backfill`, with optional status/export refreshes |

All wrappers honor `STELLAR_FORGE_BIN` if you want them to invoke a different binary name.

## Input File Examples

Several commands expect structured files. Keeping a canonical sample next to your project helps a
lot when you revisit these flows later.

### Wallet batch JSON

Used by:

- `wallet batch-pay`
- `wallet batch-validate`
- `wallet batch-preview`
- `wallet batch-summary`
- `wallet batch-report`
- `wallet batch-reconcile`
- `wallet batch-resume`

Example:

```json
[
  { "to": "alice", "amount": "10", "asset": "XLM" },
  { "to": "bob", "amount": "25", "asset": "points" },
  { "to": "carol", "amount": "5" }
]
```

Notes:

- `asset` is optional on each row
- when a row omits `asset`, the command-level `--asset` value is used
- a top-level object with a `payments` array also works

### Wallet batch CSV

```csv
to,amount,asset
alice,10,XLM
bob,25,points
carol,5,
```

Useful commands:

```bash
stellar forge wallet batch-preview --from treasury --asset XLM --file payouts.csv --format csv
stellar forge wallet batch-report --from treasury --asset XLM --file payouts.csv --format csv
stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.csv --format csv --report dist/payouts.report.json
```

### Token airdrop CSV

Token airdrop commands reuse the batch format, but the token name supplies the asset:

```csv
to,amount
alice,10
bob,20
carol,5
```

Useful commands:

```bash
stellar forge token airdrop points --from treasury --file rewards.csv --format csv
stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv
stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json
```

### Smart-wallet policy TOML

Used by `wallet smart policy apply` and `wallet smart policy simulate`.

```toml
source = "alice"
daily_limit = 1250
allow = ["treasury", "issuer"]
revoke = ["legacy-signer"]
build_only = true
```

Equivalent JSON:

```json
{
  "source": "alice",
  "daily_limit": 1250,
  "allow": ["treasury", "issuer"],
  "revoke": ["legacy-signer"],
  "build_only": true
}
```

### Events export and replay payload

`events export` writes a replayable JSON file. A typical workflow is:

```bash
stellar forge events export --path dist/events.json --out dist/events.export.json
stellar forge events replay --path dist/events.json --out dist/events.replay.json
```

Use this when you want to capture recent local event-store state before reseeding, debugging a
worker, or sharing a reproducible fixture with a teammate.

## Cookbook Examples

### Validate, sync, and produce an audit trail

```bash
stellar forge --json project validate --out dist/project.validate.json
stellar forge --json project sync --out dist/project.sync.json
stellar forge --json doctor audit --out dist/doctor.audit.json
```

### Preview a release without touching the network

```bash
stellar forge --dry-run --json release plan testnet --out dist/release.plan.json
stellar forge --dry-run --json release verify testnet --out dist/release.verify.json
```

### Compare current release state against history

```bash
stellar forge release status testnet --out dist/release.status.json
stellar forge release diff testnet --out dist/release.diff.json
stellar forge release inspect testnet --path dist/history/deploy.testnet.20260413T000000.000Z.json --out dist/release.inspect.json
stellar forge release drift testnet --out dist/release.drift.json
```

### Smart-wallet local maintenance loop

```bash
stellar forge wallet smart info checkout-passkey --out dist/wallet.smart.info.json
stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json
stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json
stellar forge wallet smart policy simulate checkout-passkey --file policy.toml --out dist/wallet.smart.policy.simulate.json
```

## Report Anatomy

When you use `--json`, most commands follow the same broad report shape:

| Field | Meaning |
| --- | --- |
| `action` | normalized command id such as `release.plan` or `wallet.batch-report` |
| `status` | overall result: usually `ok`, `warn`, or `error` |
| `network` | active network when the command is network-aware |
| `checks` | structured checks produced during validation or verification |
| `commands` | underlying shell or Stellar CLI invocations |
| `warnings` | non-fatal issues worth reading before you proceed |
| `artifacts` | files created or rewritten |
| `next` | suggested follow-up steps |
| `data` | command-specific details intended for scripting or deeper inspection |

That consistency is what makes it practical to chain commands in CI while still giving humans a
useful audit trail.
