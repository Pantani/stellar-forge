# Deployment Guide

This guide covers how `stellar-forge` moves from local development to a real network.

## Mental Model

There are three layers involved in a deploy:

1. the manifest says what should exist
2. the lockfile records what was actually deployed
3. deploy artifacts export runtime-facing values for apps and operators

The key commands are:

```bash
stellar forge release plan <env>
stellar forge release deploy <env>
stellar forge release verify <env>
stellar forge release env export <env>
stellar forge release aliases sync <env>
```

## Before You Deploy Anywhere

Run this baseline checklist:

```bash
stellar forge doctor
stellar forge project validate
stellar forge contract build
stellar forge --dry-run release plan <env>
```

Also verify:

- the target network exists in `stellarforge.toml`
- the target identity is available through the Stellar CLI
- the manifest contains every contract and token you actually intend to ship
- Docker is available if you are deploying to `local`
- API/front-end consumers know whether they should read `.env.generated` or `dist/deploy.<env>.json`

If the workspace includes a generated frontend, run the smoke checks before release work moves any
further:

```bash
stellar forge project smoke
stellar forge project smoke --browser
```

Use `project smoke` for the lighter repo-level check. Add `--browser` when you want the browser
runner to exercise the frontend in Playwright as part of release readiness. On fresh machines or
clean worktrees, `--install` is the safest version because it adds the package-manager install step
before the smoke run.

## Preflight Matrix

Use this as a quick "am I ready?" grid before touching a real environment:

| Check | Local | Testnet | Futurenet | Pubnet |
| --- | --- | --- | --- | --- |
| `stellar forge doctor` | recommended | required | required | required |
| `stellar forge project validate` | required | required | required | required |
| `stellar forge contract build` | usually required | required | required | required |
| `stellar forge project smoke` | useful for generated apps | recommended | recommended | strongly recommended |
| `stellar forge project smoke --browser` | useful for frontend changes | recommended before release | recommended before release | strongly recommended before release |
| `stellar forge --dry-run release plan <env>` | recommended | required | required | required |
| dedicated deploy identity checked | optional | required | required | required |
| friendbot availability expected | n/a | yes | environment-specific | no |
| `--confirm-mainnet` required | no | no | no | yes |

If any of the required checks fail, fix that first. A deploy plan is only as trustworthy as the
workspace and local toolchain it is reading from.

## Environment Strategy

### Local

Best for rapid iteration, reseeding, and validating the generated project layout.

Commands:

```bash
stellar forge dev up
stellar forge dev reseed --network local
stellar forge dev status
stellar forge release verify local
stellar forge project smoke --browser
```

For frontend changes, use the browser smoke scripts in `apps/web` directly when you want to split
the cost across steps:

```bash
pnpm --dir apps/web smoke:browser:build
pnpm --dir apps/web smoke:browser:install
pnpm --dir apps/web smoke:browser:run
pnpm --dir apps/web smoke:browser
```

Recommended use:

- `smoke:browser:build` after UI or generated frontend changes
- `smoke:browser:install` on a new machine or after clearing the Playwright cache
- `smoke:browser:run` when the build is already in place and you only need the browser assertion
- `smoke:browser` when you want the complete browser smoke in one command

The full `smoke:browser` flow checks whether the pinned Chromium bundle is already present in the
Playwright cache. If it is, the install step is skipped. If not, Chromium is installed before the
browser test runs. If port `4173` is already in use, rerun with
`STELLAR_FORGE_BROWSER_SMOKE_PORT=<port>`.

What happens:

- `dev up` starts `stellar container start local`
- `dev reseed` recreates identities, tokens, contracts, env exports, and event state
- local deploy artifacts reflect the current local container state

### Testnet

Best for rehearsal against a shared public environment with friendbot funding.

Commands:

```bash
stellar forge doctor network testnet
stellar forge release plan testnet
stellar forge release deploy testnet
stellar forge release verify testnet
stellar forge release env export testnet
```

Recommended testnet sequence:

1. build locally
2. make sure required identities exist
3. fund new identities if needed with `stellar forge wallet fund <name>`
4. run `release plan`
5. run `release deploy`
6. run `release verify`
7. export env and restart consuming apps if needed

### Futurenet

Same overall flow as testnet, but treat it as a pre-release sandbox that may change underneath
you more often.

Commands:

```bash
stellar forge release plan futurenet
stellar forge release deploy futurenet
stellar forge release verify futurenet
```

### Pubnet / mainnet

Only do this once your manifest, identities, and release scope are explicit.

You usually want a dedicated network entry, for example:

```toml
[networks.pubnet]
kind = "pubnet"
rpc_url = "<your pubnet RPC URL>"
horizon_url = "<your Horizon URL>"
network_passphrase = "Public Global Stellar Network ; September 2015"
allow_http = false
friendbot = false

[release.pubnet]
deploy_contracts = ["rewards"]
deploy_tokens = ["points"]
generate_env = true
```

Mainnet commands:

```bash
stellar forge doctor network pubnet
stellar forge release plan pubnet
stellar forge release deploy pubnet --confirm-mainnet
stellar forge release verify pubnet
stellar forge release env export pubnet
stellar forge release aliases sync pubnet
```

The `--confirm-mainnet` flag is mandatory by design.

## Zero-To-Testnet Playbook

This is the shortest full rehearsal loop from a fresh checkout or new scaffold:

```bash
stellar forge doctor
stellar forge project validate
stellar forge project sync
stellar forge contract build
stellar forge project smoke
stellar forge --dry-run release plan testnet
stellar forge release deploy testnet
stellar forge release verify testnet
stellar forge release env export testnet
stellar forge release aliases sync testnet
```

What to inspect after each phase:

- after `doctor`: external tooling and plugin visibility
- after `project validate`: manifest correctness and generated-file drift
- after `contract build`: contract workspaces still compile under the current local toolchain
- after `project smoke`: generated frontend and API glue still works
- after `release plan`: release scope, identities, and expected artifacts match intent
- after `release deploy`: `stellarforge.lock.json` changed only in the ways you expected
- after `release verify`: deploy artifacts line up with the live environment

## Release Scope

`stellar-forge` determines release scope like this:

- if `[release.<env>]` exists, it deploys only `deploy_contracts` and `deploy_tokens`
- if that block does not exist, it falls back to every declared contract and token

That means production deploys should usually define `[release.pubnet]` or `[release.mainnet]`
explicitly, even if lower environments use the implicit fallback.

## What `release plan` Shows You

`release plan <env>` is your audit step before anything mutates state. It assembles:

- manifest validation results
- required identities
- preview commands that will be executed
- expected lockfile changes
- whether `.env.generated` will be written
- which artifacts will appear in `dist/`
- registry-based alternatives when registry metadata already exists

Use it heavily:

```bash
stellar forge --dry-run release plan testnet
stellar forge --json release plan testnet
```

## What `release deploy` Actually Does

At a high level:

1. materializes tokens that belong in the release
2. deploys contracts included in the release
3. runs contract init flows when declared
4. updates `stellarforge.lock.json`
5. writes `.env.generated` and `dist/deploy.<env>.json` when the release requires env export

For classic asset tokens, this may mean asset creation and optional SAC handling.
For contract tokens, this may involve deploying and initializing the matching contract.

## Post-Deploy Verification

Run this immediately after deploy:

```bash
stellar forge release verify <env>
```

What it checks:

- deploy state exists in the lockfile
- `.env.generated` matches the current manifest and lockfile
- `dist/deploy.<env>.json` matches the current manifest and lockfile
- registry artifacts align with deploy state
- event worker config is present when needed
- live on-chain fetch probes for contract IDs, when not in `--dry-run` and `stellar` is available

Also useful:

```bash
stellar forge doctor network <env>
stellar forge project info
```

## Using `status`, `drift`, `diff`, `history`, And `inspect` Together

The release-inspection commands are easiest to understand as a layered toolkit:

| Command | Best question it answers |
| --- | --- |
| `release status <env>` | what does the current snapshot look like, and is there archived history? |
| `release drift <env>` | how far is the current workspace from the expected release state? |
| `release diff <env>` | how does the current snapshot compare to a selected archived snapshot? |
| `release history <env>` | what snapshots are available under `dist/history/`? |
| `release inspect <env>` | what is inside one snapshot, and how does it compare to the current state? |

Recommended troubleshooting flow:

```bash
stellar forge release status testnet --out dist/release.status.json
stellar forge release drift testnet --out dist/release.drift.json
stellar forge release history testnet --out dist/release.history.json
stellar forge release inspect testnet --path dist/history/deploy.testnet.20260413T000000.000Z.json --out dist/release.inspect.json
stellar forge release diff testnet --path dist/history/deploy.testnet.20260413T000000.000Z.json --out dist/release.diff.json
```

Use that sequence when:

- teammates report different contract ids locally
- `.env.generated` looks stale
- the lockfile changed unexpectedly
- you want to understand whether drift comes from the live network or only from local artifacts

## Rollbacking Local Release Metadata

Every time `dist/deploy.<env>.json` is replaced with new contents, the previous artifact is copied
into `dist/history/`. You can restore local release metadata from one of those snapshots with:

```bash
stellar forge release history <env>
stellar forge release inspect <env>
stellar forge release rollback <env>
stellar forge release rollback <env> --to dist/history/deploy.<env>.<timestamp>.json
```

This updates:

- `stellarforge.lock.json`
- `.env.generated`
- `dist/deploy.<env>.json`

It is a local-state rollback, not an on-chain rollback. After restoring, rerun:

```bash
stellar forge release verify <env>
stellar forge release aliases sync <env>
```

## Exported Artifacts

### `.env.generated`

Exported by `release env export` and some deploy flows.

Contains values such as:

- `PUBLIC_STELLAR_NETWORK`
- `PUBLIC_STELLAR_RPC_URL`
- `PUBLIC_<NAME>_CONTRACT_ID`
- `PUBLIC_<NAME>_ASSET`
- `PUBLIC_<NAME>_SAC_ID`
- `PUBLIC_<NAME>_TOKEN_ID`

Use this file when generated apps need runtime IDs and asset strings.

### `dist/deploy.<env>.json`

A release snapshot that includes:

- project metadata
- environment name
- network metadata
- deployed contract entries
- deployed token entries
- whether env generation was requested

This file is the easiest machine-readable handoff to deployment tooling and external services.

### `dist/registry.<env>.json`

Created and maintained by registry publish/deploy flows. It keeps registry-oriented metadata such
as published references, Wasm hashes, aliases, and deployed IDs.

## Alias Synchronization

After deploy, you can sync local Stellar CLI aliases from the manifest and lockfile:

```bash
stellar forge release aliases sync testnet
```

This is especially useful after fresh machines, lockfile imports, or when teammates need matching
contract aliases locally.

## Registry Workflows

`stellar-forge` supports two registry execution backends:

- `stellar registry ...` from the installed `stellar` CLI
- a dedicated `stellar-registry` binary

Control it with:

```bash
export STELLAR_FORGE_REGISTRY_MODE=stellar
export STELLAR_FORGE_REGISTRY_MODE=dedicated
```

Typical flow:

```bash
stellar forge --network testnet release registry publish rewards
stellar forge --network testnet release registry deploy rewards
```

Use registry flows when your contract distribution model depends on published Wasm references
rather than only direct contract deploys.

Registry selection tips:

- use the default auto-detection when you simply want the available backend
- set `STELLAR_FORGE_REGISTRY_MODE=stellar` when you need parity with the official `stellar` CLI
- set `STELLAR_FORGE_REGISTRY_MODE=dedicated` when your environment standardizes on the standalone
  `stellar-registry` binary

## Suggested Deployment Playbooks

### Local development reset

```bash
stellar forge dev up
stellar forge dev reseed --network local
stellar forge release verify local
```

### First testnet release

```bash
stellar forge doctor
stellar forge contract build
stellar forge wallet create deployer --fund
stellar forge --identity deployer release plan testnet
stellar forge --identity deployer release deploy testnet
stellar forge --identity deployer release verify testnet
stellar forge release env export testnet
```

### Production release

```bash
stellar forge doctor
stellar forge project validate
stellar forge contract build --optimize
stellar forge --dry-run release plan pubnet
stellar forge release deploy pubnet --confirm-mainnet
stellar forge release verify pubnet
stellar forge release env export pubnet
stellar forge release aliases sync pubnet
```

## Failure Modes And Recovery

### Lockfile no longer matches the network

Symptoms:

- `release verify` fails fetch probes
- `doctor network <env>` warns that IDs no longer resolve

Typical fixes:

- redeploy the affected environment
- rerun `release env export <env>`
- commit the resulting lockfile and deploy artifact updates

### Public test environment reset

Testnet and futurenet can lose deployed state relative to your lockfile.

Typical fixes:

- rerun `release deploy <env>`
- or rerun `dev reseed --network <env>` if you are using that environment as a managed sandbox

### Registry tooling not available

Symptoms:

- `release plan` warns that registry alternatives exist but the backend is unavailable

Typical fixes:

- install a `stellar` CLI version that exposes `stellar registry`
- or install `stellar-registry`
- or set `STELLAR_FORGE_REGISTRY_MODE` so the expected backend is explicit

### Apps are using stale IDs

Symptoms:

- API or frontend still points at old contract IDs

Typical fixes:

- rerun `stellar forge release env export <env>`
- restart `apps/api` and `apps/web`
- verify `.env.generated` and `dist/deploy.<env>.json` reflect the current lockfile

## Operational Checklist

Use this before finishing a release:

```bash
stellar forge project validate
stellar forge release plan <env>
stellar forge release deploy <env>
stellar forge release verify <env>
stellar forge release env export <env>
stellar forge release aliases sync <env>
```

Then review together:

- `stellarforge.toml`
- `stellarforge.lock.json`
- `.env.generated`
- `dist/deploy.<env>.json`
- `dist/registry.<env>.json` when registry flows were used

## What To Commit After A Release

Treat the release as both an on-chain change and a repository-state change.

Normally you want to review and commit:

- `stellarforge.toml` when release scope or manifest intent changed
- `stellarforge.lock.json`
- `.env.generated` if your workflow tracks exported runtime ids in git
- `dist/deploy.<env>.json`
- `dist/registry.<env>.json` when registry flows were used

If the release also required regeneration, include the updated generated files in the same review so
the repo reflects the deploy you actually verified.
