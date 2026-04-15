# Documentation

This folder is the long-form reference for `stellar-forge`.

Use it as a map, not just as a table of contents: each document answers a different class of
question.

## Suggested Reading Order

1. [README](../README.md) for the project overview, install, template chooser, and common flows
2. [Command reference](command-reference.md) for the exact CLI surface and command examples
3. [Manifest and state reference](manifest-reference.md) for `stellarforge.toml`,
   `stellarforge.lock.json`, symbolic references, generated files, and environment variables
4. [Deployment guide](deployment-guide.md) for local/testnet/pubnet release workflows, verification,
   rollback, and drift recovery

## Which Doc Should I Read?

| If you need to... | Start here | Why |
| --- | --- | --- |
| create a new project | [README](../README.md#quick-start) | it gets you from `init` to a working local loop quickly |
| compare commands and flags | [command-reference.md](command-reference.md) | it is the canonical syntax reference |
| understand how the manifest drives the workspace | [manifest-reference.md](manifest-reference.md) | it explains the config model and file relationships |
| ship a release safely | [deployment-guide.md](deployment-guide.md) | it focuses on plan, deploy, verify, artifacts, and recovery |
| debug drift between manifest, lockfile, and generated outputs | [manifest-reference.md](manifest-reference.md#which-commands-update-which-files) | it shows what each command is expected to rewrite |
| recover from a broken or stale deploy snapshot | [deployment-guide.md](deployment-guide.md#using-status-drift-diff-history-and-inspect-together) | it explains the release history and comparison commands |
| prepare CI-friendly report files | [command-reference.md](command-reference.md#how-to-use-this-reference) | it covers `--json`, `--out`, and report anatomy |

## Reading Paths By Job

### New project

1. [README quick start](../README.md#quick-start)
2. [Command reference: `init`](command-reference.md#init)
3. [Manifest example](manifest-reference.md#example-manifest)
4. [Deployment guide: local](deployment-guide.md#local)

### Existing workspace or scaffold adoption

1. [Command reference: `project`](command-reference.md#project)
2. [Manifest reference: validation rules](manifest-reference.md#validation-rules-worth-knowing)
3. [Deployment guide: verification](deployment-guide.md#post-deploy-verification)

### Smart-wallet and policy flows

1. [Command reference: `wallet`](command-reference.md#wallet)
2. [Manifest reference: smart-wallet example](manifest-reference.md#smart-wallet-example)
3. [README input file examples](../README.md#input-file-examples)

### Release and recovery

1. [Deployment guide](deployment-guide.md)
2. [Command reference: `release`](command-reference.md#release)
3. [Manifest reference: lockfile structure](manifest-reference.md#lockfile-structure)

## Common Files

| File or directory | What it means | Best doc |
| --- | --- | --- |
| `stellarforge.toml` | desired project state | [manifest-reference.md](manifest-reference.md) |
| `stellarforge.lock.json` | observed deployed state per environment | [manifest-reference.md](manifest-reference.md#lockfile-structure) |
| `.env.example` | manifest-derived defaults | [manifest-reference.md](manifest-reference.md#generated-files-and-artifacts) |
| `.env.generated` | release or dev-exported runtime values | [deployment-guide.md](deployment-guide.md#exported-artifacts) |
| `dist/deploy.<env>.json` | machine-readable release artifact | [deployment-guide.md](deployment-guide.md#exported-artifacts) |
| `dist/history/` | archived release snapshots | [deployment-guide.md](deployment-guide.md#rollbacking-local-release-metadata) |
| `workers/events/` | event ingest helpers and cursor snapshot | [command-reference.md](command-reference.md#events) |
| `scripts/` | generated wrappers for doctor, reseed, and release flows | [command-reference.md](command-reference.md#generated-helper-scripts) |

## Fast Answers

### I just want to bootstrap a project

Start with:

```bash
stellar forge init hello-stellar --template fullstack --network testnet
cd hello-stellar
stellar forge doctor
stellar forge project validate
stellar forge dev up
```

### I want the full command surface

Go to [command-reference.md](command-reference.md).

### I want to understand the config model

Go to [manifest-reference.md](manifest-reference.md).

### I need a deploy checklist

Go to [deployment-guide.md](deployment-guide.md).

### I want examples of batch files or policy files

Start in [README input file examples](../README.md#input-file-examples), then use the matching
sections in [command-reference.md](command-reference.md).
