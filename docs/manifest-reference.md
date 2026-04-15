# Manifest And State Reference

`stellar-forge` revolves around two project files:

- `stellarforge.toml`: desired state and project layout
- `stellarforge.lock.json`: observed deployment state per environment

The manifest is edited by humans. The lockfile is updated by commands such as deploy, token create,
SAC deploy, and adoption flows.

## High-Level Model

The normal loop is:

1. declare or edit project intent in `stellarforge.toml`
2. run `stellar forge project validate`
3. run `stellar forge project sync`
4. run operational commands such as `contract deploy`, `token create`, or `release deploy`
5. commit both the manifest and the lockfile when the resulting state is intentional

## What Lives Where

The project state is intentionally split across a few files with different responsibilities:

| File | Role | Edited by |
| --- | --- | --- |
| `stellarforge.toml` | desired project structure and release intent | humans and selected scaffold commands |
| `stellarforge.lock.json` | observed deploy state per environment | deploy, token, adoption, and rollback flows |
| `.env.example` | manifest-derived defaults | `init`, `project sync`, `doctor fix` |
| `.env.generated` | runtime values derived from actual deployed state | `dev up`, `dev reseed`, `release env export`, `release deploy` |
| `dist/deploy.<env>.json` | machine-readable deploy snapshot | `release env export`, `release deploy`, rollback helpers |
| `workers/events/cursors.json` | local event cursor snapshot | `init`, event setup, `events backfill`, `dev snapshot` |

## Example Manifest

```toml
[project]
name = "rewards-app"
slug = "rewards-app"
version = "0.1.0"
package_manager = "pnpm"

[defaults]
network = "testnet"
identity = "alice"
output = "human"

[networks.local]
kind = "local"
rpc_url = "http://localhost:8000/rpc"
horizon_url = "http://localhost:8000"
network_passphrase = "Standalone Network ; February 2017"
allow_http = true
friendbot = true

[networks.testnet]
kind = "testnet"
rpc_url = "https://soroban-testnet.stellar.org"
horizon_url = "https://horizon-testnet.stellar.org"
network_passphrase = "Test SDF Network ; September 2015"
allow_http = false
friendbot = true

[identities.alice]
source = "stellar-cli"
name = "alice"

[identities.issuer]
source = "stellar-cli"
name = "issuer"

[wallets.alice]
kind = "classic"
identity = "alice"

[wallets.issuer]
kind = "classic"
identity = "issuer"

[tokens.points]
kind = "asset"
code = "POINTS"
issuer = "@identity:issuer"
distribution = "@identity:issuer"
auth_required = true
auth_revocable = true
clawback_enabled = true
with_sac = true
decimals = 7
metadata_name = "Loyalty Points"

[contracts.rewards]
path = "contracts/rewards"
alias = "rewards"
template = "rewards"
bindings = ["typescript"]
deploy_on = ["local", "testnet"]

[contracts.rewards.init]
fn = "init"
admin = "@identity:issuer"
token = "@token:points:sac"

[api]
enabled = true
framework = "fastify"
database = "sqlite"
events_backend = "rpc-poller"
openapi = true
relayer = false

[frontend]
enabled = true
framework = "react-vite"

[release.testnet]
deploy_contracts = ["rewards"]
deploy_tokens = ["points"]
generate_env = true

[scenarios.checkout]
description = "Dry-run the checkout happy path"
network = "testnet"
identity = "alice"

[[scenarios.checkout.steps]]
action = "project.validate"

[[scenarios.checkout.steps]]
action = "contract.call"
contract = "rewards"
function = "award_points"
build_only = true
args = ["--member", "alice", "--amount", "25"]

[[scenarios.checkout.steps]]
action = "wallet.pay"
from = "treasury"
to = "alice"
asset = "points"
amount = "10"
build_only = true

[[scenarios.checkout.steps]]
action = "release.plan"

[[scenarios.checkout.assertions]]
assertion = "status"
status = "ok"

[[scenarios.checkout.assertions]]
assertion = "step"
step = 2
status = "ok"
command_contains = ["stellar contract invoke", "award_points"]
```

## Minimal Contract-Only Manifest

This is the smallest useful shape for a project that mainly wants contract lifecycle management:

```toml
[project]
name = "hello-contract"
slug = "hello-contract"
version = "0.1.0"
package_manager = "pnpm"

[defaults]
network = "testnet"
identity = "alice"
output = "human"

[networks.testnet]
kind = "testnet"
rpc_url = "https://soroban-testnet.stellar.org"
horizon_url = "https://horizon-testnet.stellar.org"
network_passphrase = "Test SDF Network ; September 2015"
allow_http = false
friendbot = true

[identities.alice]
source = "stellar-cli"
name = "alice"

[wallets.alice]
kind = "classic"
identity = "alice"

[contracts.app]
path = "contracts/app"
alias = "app"
template = "basic"
bindings = ["typescript"]
deploy_on = ["testnet"]
```

## Smart-Wallet Example

Smart wallets are still regular manifest entries, but they add controller and policy metadata:

```toml
[wallets.checkout-passkey]
kind = "smart"
mode = "passkey"
controller_identity = "checkout-owner"
onboarding_app = "apps/smart-wallet/checkout-passkey"
policy_contract = "checkout-passkey-policy"

[identities.checkout-owner]
source = "stellar-cli"
name = "checkout-owner"

[contracts.checkout-passkey-policy]
path = "contracts/checkout-passkey-policy"
alias = "checkout-passkey-policy"
template = "smart-wallet-policy"
bindings = ["typescript"]
deploy_on = ["testnet"]
```

Typical follow-up commands:

```bash
stellar forge wallet smart onboard checkout-passkey
stellar forge wallet smart materialize checkout-passkey
stellar forge wallet smart policy diff checkout-passkey
stellar forge wallet smart policy sync checkout-passkey
```

## Manifest Sections

### `[project]`

Project metadata and package-manager choice.

| Key | Meaning |
| --- | --- |
| `name` | Human-readable project name |
| `slug` | Filesystem- and URL-friendly project identifier |
| `version` | Project version included in generated metadata |
| `package_manager` | One of `pnpm`, `npm`, `yarn`, `bun`, or another executable you manage yourself |

### `[defaults]`

Runtime defaults used when `--network` or `--identity` are not passed on the command line.

| Key | Meaning |
| --- | --- |
| `network` | Default network name |
| `identity` | Default source identity |
| `output` | Default output mode, usually `human` |

### `[networks.<name>]`

Named network definitions. The scaffolded defaults are `local`, `testnet`, and `futurenet`.

| Key | Meaning |
| --- | --- |
| `kind` | Semantic label such as `local`, `testnet`, `futurenet`, or `pubnet` |
| `rpc_url` | Soroban RPC endpoint |
| `horizon_url` | Horizon endpoint |
| `network_passphrase` | Network passphrase used by Stellar |
| `allow_http` | Whether plain HTTP is allowed |
| `friendbot` | Whether friendbot funding should be available |

### `[identities.<name>]`

Named identities resolved through the Stellar CLI.

| Key | Meaning |
| --- | --- |
| `source` | Identity provider, default `stellar-cli` |
| `name` | External identity name the provider should resolve |

### `[wallets.<name>]`

Named wallets.

Classic wallet keys:

- `kind = "classic"`
- `identity = "<identity-name>"`

Smart wallet keys:

- `kind = "smart"`
- `mode = "ed25519"` or `mode = "passkey"`
- `controller_identity`
- `onboarding_app`
- `policy_contract`

### `[tokens.<name>]`

Named token definitions.

| Key | Meaning |
| --- | --- |
| `kind` | Usually `asset` or `contract` |
| `code` | Asset code for classic assets |
| `issuer` | Identity or wallet reference for the issuing side |
| `distribution` | Identity or wallet reference for the treasury/distribution side |
| `auth_required` | Classic asset auth_required flag |
| `auth_revocable` | Classic asset auth_revocable flag |
| `clawback_enabled` | Classic asset clawback flag |
| `with_sac` | Whether a Stellar Asset Contract wrapper is expected |
| `decimals` | Token decimal precision, default 7 |
| `metadata_name` | Human-friendly token name |

Contract-token rule: if `kind = "contract"`, a contract with the same logical name must also be
declared under `[contracts.<name>]`.

### `[contracts.<name>]`

Contract definitions and deployment defaults.

| Key | Meaning |
| --- | --- |
| `path` | Relative path to the contract workspace |
| `alias` | Stellar CLI alias to use after deploy |
| `template` | Template label used by scaffolded flows |
| `bindings` | Binding languages to generate |
| `deploy_on` | Environments where the contract is intended to deploy |

Optional nested init block:

```toml
[contracts.rewards.init]
fn = "init"
admin = "@identity:issuer"
token = "@token:points:sac"
```

This tells `release deploy` or contract-token flows how to run an initialization transaction after
upload/deploy.

### `[api]`

Configuration for the managed API scaffold.

| Key | Meaning |
| --- | --- |
| `enabled` | Whether `apps/api` should exist |
| `framework` | Current scaffold target, effectively Fastify-oriented today |
| `database` | Storage backend, typically `sqlite` |
| `events_backend` | Event ingestion mode, default `rpc-poller` |
| `openapi` | Whether OpenAPI export should be generated |
| `relayer` | Whether relayer endpoints should be scaffolded |

### `[frontend]`

Configuration for the managed frontend scaffold.

| Key | Meaning |
| --- | --- |
| `enabled` | Whether `apps/web` should exist |
| `framework` | Current scaffold target, default `react-vite` |

### `[release.<env>]`

Optional release override for a specific environment.

| Key | Meaning |
| --- | --- |
| `deploy_contracts` | Contracts included in the release |
| `deploy_tokens` | Tokens included in the release |
| `generate_env` | Whether `.env.generated` should be written during release |

If `[release.<env>]` is missing, `release plan|deploy|verify` fall back to all declared contracts
and tokens.

### `[scenarios.<name>]`

Optional named workflows that compose existing CLI primitives into a reproducible rehearsal.

Top-level keys:

| Key | Meaning |
| --- | --- |
| `description` | Human-oriented label shown in reports |
| `network` | Optional default network for the scenario |
| `identity` | Optional default identity for the scenario |
| `steps` | Ordered list of typed steps |
| `assertions` | Optional checks evaluated by `scenario test` after all preview steps finish |

Supported step actions in v1:

- `project.validate`
- `project.sync`
- `dev.up`
- `dev.reseed`
- `dev.fund`
- `contract.build`
- `contract.deploy`
- `contract.call`
- `token.mint`
- `wallet.pay`
- `release.plan`
- `release.verify`

Example:

```toml
[scenarios.local-refresh]
description = "Refresh derived files and preview a release"
network = "local"
identity = "alice"

[[scenarios.local-refresh.steps]]
action = "project.sync"

[[scenarios.local-refresh.steps]]
action = "project.validate"

[[scenarios.local-refresh.steps]]
action = "release.plan"
env = "local"

[[scenarios.local-refresh.assertions]]
assertion = "status"
status = "ok"

[[scenarios.local-refresh.assertions]]
assertion = "step"
step = 3
command_contains = ["release plan local"]
```

Run these with:

```bash
stellar forge scenario test local-refresh
stellar forge scenario run local-refresh
```

Supported assertions in v1:

- `assertion = "status"` with `status = "ok" | "warn" | "error"`
- `assertion = "step"` with a 1-based `step` plus any combination of:
  `status`, `command_contains`, `artifact_contains`, `warning_contains`

### Scenario Step Recipes

The scenario system intentionally reuses normal CLI semantics. A few common step shapes:

Validate and sync:

```toml
[[scenarios.refresh.steps]]
action = "project.validate"

[[scenarios.refresh.steps]]
action = "project.sync"
```

Build and deploy a named contract:

```toml
[[scenarios.release-check.steps]]
action = "contract.build"
contract = "rewards"
optimize = true

[[scenarios.release-check.steps]]
action = "contract.deploy"
contract = "rewards"
env = "testnet"
```

Preview a contract call without sending:

```toml
[[scenarios.checkout.steps]]
action = "contract.call"
contract = "rewards"
function = "award_points"
build_only = true
args = ["--member", "alice", "--amount", "25"]
```

Preview a payment:

```toml
[[scenarios.checkout.steps]]
action = "wallet.pay"
from = "treasury"
to = "alice"
asset = "points"
amount = "10"
build_only = true
```

Release preflight:

```toml
[[scenarios.release-check.steps]]
action = "release.plan"
env = "testnet"

[[scenarios.release-check.steps]]
action = "release.verify"
env = "testnet"
```

## Manifest Reference Syntax

Several manifest fields accept symbolic references instead of raw addresses or IDs.

Supported forms:

| Syntax | Resolves to |
| --- | --- |
| `@identity:<name>` | A named identity |
| `@wallet:<name>` | A named wallet |
| `@token:<name>` | A named token |
| `@token:<name>:sac` | The SAC contract for a token |
| `@contract:<name>` | A named contract |

Typical places you will use these:

- token issuer and distribution fields
- contract init arguments
- release arguments derived from contract init config

Examples:

```toml
issuer = "@identity:issuer"
distribution = "@wallet:treasury"
token = "@token:points:sac"
admin_contract = "@contract:rewards"
```

How to read them:

- `@identity:issuer` resolves through `[identities.issuer]`
- `@wallet:treasury` resolves through `[wallets.treasury]`
- `@token:points:sac` means "the deployed SAC wrapper for token `points`"
- `@contract:rewards` means "the deployed contract id for `rewards` in the active environment"

## Validation Rules Worth Knowing

The CLI validates, among other things:

- project, network, wallet, token, contract, and release keys must be filesystem-safe names
- `defaults.network` and `defaults.identity` must point to declared entries
- contract paths must stay inside the project root
- classic wallets must reference existing identities
- smart wallets must include `mode` and valid controller/policy references
- contract tokens must have a matching contract entry

Run:

```bash
stellar forge project validate
stellar forge doctor project
```

## Lockfile Structure

`stellarforge.lock.json` stores materialized state per environment.

Typical shape:

```json
{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {
        "rewards": {
          "contract_id": "C...",
          "alias": "rewards",
          "wasm_hash": "...",
          "tx_hash": "...",
          "deployed_at": "2026-04-14T00:00:00Z"
        }
      },
      "tokens": {
        "points": {
          "kind": "asset",
          "asset": "POINTS:G...",
          "issuer_identity": "issuer",
          "distribution_identity": "treasury",
          "sac_contract_id": "C...",
          "contract_id": ""
        }
      }
    }
  }
}
```

The lockfile is how later commands resolve:

- deployed contract IDs
- SAC IDs
- aliases
- Wasm hashes
- environment-specific token materialization

## Generated Files And Artifacts

### Files derived from the manifest

| Path | Produced by |
| --- | --- |
| `.env.example` | `init`, `project sync` |
| `apps/api/**` | `init`, `api init`, `project sync`, `api generate`, `api events init`, `api relayer init` |
| `apps/web/**` | `init`, `project sync`, frontend-related sync flows |
| `packages/**` | `contract bind`, contract-token flows |
| `workers/events/**` | `init`, event scaffold flows |

### Files derived from deployment state

| Path | Produced by |
| --- | --- |
| `.env.generated` | `dev up`, `dev reseed`, `release env export`, some release deploy flows |
| `dist/deploy.<env>.json` | `release env export`, `release deploy` |
| `dist/registry.<env>.json` | Registry publish/deploy flows |
| `dist/contracts/<name>.<env>.wasm` | `contract fetch` |

## Which Commands Update Which Files

When you are reviewing a diff, this table helps you decide whether the changed files make sense:

| Command family | Usually updates |
| --- | --- |
| `project sync` | `.env.example`, API/frontend scaffold files, generated state, OpenAPI output |
| `doctor fix --scope scripts` | `scripts/doctor.mjs`, `scripts/reseed.mjs`, `scripts/release.mjs` |
| `doctor fix --scope api` | `apps/api/**`, `apps/api/openapi.json` |
| `doctor fix --scope frontend` | `apps/web/**`, generated frontend state |
| `contract bind` | `packages/**` |
| `events backfill` | local sqlite store plus `workers/events/cursors.json` |
| `release deploy` | `stellarforge.lock.json`, `.env.generated`, `dist/deploy.<env>.json` |
| `release env export` | `.env.generated`, `dist/deploy.<env>.json` |
| `release rollback` | `stellarforge.lock.json`, `.env.generated`, `dist/deploy.<env>.json` |
| `dev snapshot save|load` | snapshot artifacts and restored local state files |

## Environment Variables

### Used directly by the CLI

| Variable | Meaning |
| --- | --- |
| `STELLAR_FORGE_REGISTRY_MODE` | Force registry backend selection: `stellar` or `dedicated` |
| `STELLAR_FORGE_API_URL` | Base URL used for relayer submission |
| `PUBLIC_STELLAR_API_URL` | Alternate relayer base URL lookup |
| `PORT` | Fallback port when inferring a local API URL |
| `STELLAR_FORGE_BIN` | Override executable name used by generated helper scripts |

### Used by generated relayer/API scaffolds

| Variable | Meaning |
| --- | --- |
| `RELAYER_BASE_URL` | Upstream relayer base URL |
| `RELAYER_API_KEY` | Relayer auth key |
| `RELAYER_SUBMIT_PATH` | Submission path, defaulted inside the scaffold |
| `STELLAR_EVENTS_DB_PATH` | Event sqlite path |
| `STELLAR_EVENTS_POLL_INTERVAL_MS` | Poll cadence for the worker |
| `STELLAR_EVENTS_BATCH_SIZE` | Batch size |
| `STELLAR_EVENTS_START_LEDGER` | Backfill starting ledger |
| `STELLAR_EVENTS_RESOURCES` | Comma-separated tracked resources |
| `STELLAR_EVENTS_TOPICS` | Topic filters |
| `STELLAR_EVENTS_TYPE` | Event type selector |
| `STELLAR_EVENTS_RETENTION_DAYS` | Retention warning baseline |

### Exported into `.env.generated`

| Variable | Meaning |
| --- | --- |
| `PUBLIC_STELLAR_NETWORK` | Active release environment |
| `PUBLIC_STELLAR_RPC_URL` | RPC URL for that environment |
| `PUBLIC_<NAME>_CONTRACT_ID` | Contract ID for a deployed contract |
| `PUBLIC_<NAME>_ASSET` | Classic asset string for a deployed token |
| `PUBLIC_<NAME>_SAC_ID` | SAC contract ID for a deployed token |
| `PUBLIC_<NAME>_TOKEN_ID` | Contract-token ID for a deployed token |

## Working Safely

Good default loop:

```bash
stellar forge project validate
stellar forge --dry-run release plan testnet
stellar forge release deploy testnet
stellar forge release verify testnet
```

Treat `stellarforge.toml`, `stellarforge.lock.json`, `.env.generated`, and `dist/deploy.<env>.json`
as a coherent set whenever a release changes.

Editing checklist:

1. edit `stellarforge.toml`
2. run `stellar forge project validate`
3. run `stellar forge project sync`
4. run the narrow operational command you actually changed for
5. inspect `stellarforge.lock.json`, `.env.generated`, and `dist/deploy.<env>.json` if deploy state changed
6. commit the declarative and generated files together when the outcome is intentional
