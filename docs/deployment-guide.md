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

## Environment Strategy

### Local

Best for rapid iteration, reseeding, and validating the generated project layout.

Commands:

```bash
stellar forge dev up
stellar forge dev reseed --network local
stellar forge dev status
stellar forge release verify local
```

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
