---
name: live-stellar-smoke-testing
description: Use for stellar-forge live or near-live Stellar CLI validation, plugin discovery, real `stellar` command boundaries, Docker/local network smoke, testnet/pubnet checks, release deploy/verify smoke, and reruns or updates of live smoke evidence. Trigger when requests mention real Stellar, live chain, local network, Docker, testnet, pubnet, or production-like validation.
---

# Live Stellar Smoke Testing

## Purpose

Validate that `stellar-forge` works as an orchestration layer on top of the installed official `stellar` CLI. This skill is for real-tool boundaries and must not be used as a default PR gate.

## Guardrails

- Require explicit live intent before commands that can touch a network, Docker local network, or funded account.
- Run dry-run and local-only checks first.
- Never print or persist secrets, seed phrases, private keys, or unredacted funded-account details.
- Verify the installed `stellar` CLI help/version before depending on syntax that may drift.
- Treat missing `stellar`, Docker, RPC access, or funding as an environment gate.

## Workflow

1. Read `README.md`, `docs/command-reference.md`, `docs/deployment-guide.md`, and relevant command implementation.
2. Probe tools:
   - `rtk which stellar`
   - `rtk stellar --version`
   - `rtk stellar plugin ls`
   - `rtk docker --version`
3. Build the local binary:
   - `rtk cargo build --locked --bin stellar-forge`
4. Create a temp generated workspace with a template matching the requested scenario.
5. Run non-live checks first:
   - `stellar-forge doctor`
   - `stellar-forge project validate`
   - `stellar-forge --dry-run release plan <network>`
6. For local-network or testnet smoke, run only the smallest command sequence that proves the boundary.
7. Write evidence to `_workspace/test-harness/live-stellar-smoke.md`.

## Smoke Boundaries

| Boundary | What to prove |
| --- | --- |
| Plugin discovery | The official `stellar` CLI can discover or call the `forge` plugin path |
| Doctor/project validation | Environment diagnostics and manifest checks agree |
| Contract/release dry run | Planned Stellar CLI commands are generated without side effects |
| Docker/local network | Local network commands reach the expected dev boundary |
| Testnet/pubnet | Deploy/verify commands work only with explicit account and funding readiness |

## Failure Classification

Classify every failure before fixing:

- `stellar-forge` product bug
- official `stellar` CLI behavior or version drift
- Docker/local-network setup
- RPC/network availability
- account funding or identity setup
- local environment dependency

## Test Scenarios

Normal flow: user asks to validate real Stellar CLI behavior. Probe tools, build the binary, run dry-run planning in a temp workspace, then run the smallest approved live smoke.

Error flow: `stellar plugin ls` or Docker is unavailable. Record the missing tool, keep offline integration evidence, and do not claim live validation.
