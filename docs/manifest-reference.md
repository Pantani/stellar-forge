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
