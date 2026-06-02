---
name: heavy-integration-test-lab
description: Use for stellar-forge high-confidence validation, heavy integration, full-stack validation, real generated workspace testing, broad smoke matrices, local heavy labs, live/manual test tiers, environment probes, and reruns or updates of integration evidence. This skill must tier tests into offline, heavy local, and live/manual gates before execution.
---

# Heavy Integration Test Lab

## Purpose

Turn broad confidence requests into a tiered, evidence-driven integration lab. The goal is to exercise real user workflows without making normal PR verification depend on fragile local or network state.

## Test Tiers

| Tier | Purpose | Default gate |
| --- | --- | --- |
| PR-safe offline | Deterministic Rust and fake-tool coverage | Pull request and local iteration |
| Heavy local | Real generated stack, package manager, browser, SQLite, and broad matrices | Local before major handoff or scheduled CI |
| Live/manual | Real `stellar` CLI, Docker/local network, testnet/pubnet smoke | Manual or explicitly scheduled |

Never mix tier results into one generic pass/fail claim. Report each tier separately.

## Workflow

1. Read `AGENTS.MD`, `CLAUDE.MD`, `CONTRIBUTING.md`, docs, and prior `_workspace/test-harness/` notes.
2. Probe tools before running heavy work:
   - `rtk cargo --version`
   - `rtk rustc --version`
   - `rtk which stellar`
   - `rtk node --version`
   - `rtk corepack --version`
   - `rtk pnpm --version`
   - `rtk docker --version`
   - `rtk sqlite3 --version`
3. Build a matrix by command family, template, external tool, report contract, and stateful artifact.
4. Run the narrowest tier that can answer the user's confidence question.
5. Store detailed evidence under `_workspace/test-harness/05_heavy_integration_lab.md`.
6. Patch product code or tests only after identifying the failing boundary.
7. Finish with tiered verification commands and remaining gates.

## PR-Safe Offline Baseline

Use this tier for normal changes and before promoting heavier work:

```bash
rtk cargo fmt --all --check
rtk cargo check --locked --all-targets --all-features
rtk cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
rtk cargo test --locked
```

Focused offline integration commands:

```bash
rtk cargo test --locked --test cli_command_matrix
rtk cargo test --locked --test generated_workspace_sync
rtk cargo test --locked --test project_smoke_browser
rtk cargo test --locked --test events_backfill
```

These tests should use fake binaries, temp workspaces, dry runs, or dependency skips when the product contract does not require real network calls.

## Heavy Local Lab

Use this tier when the user asks for high-confidence validation, broad validation, release confidence, or generated app confidence:

- Build the CLI binary with locked dependencies.
- Generate at least one temp workspace from a full-stack template.
- Run `project validate`, `project sync`, `doctor fix`, and smoke commands.
- Use real Node/package-manager/browser/SQLite tools when present.
- Re-run stateful operations to prove idempotence.
- Keep chain-facing calls dry-run or fake unless the live tier is explicitly selected.

Useful commands:

```bash
rtk cargo build --locked --bin stellar-forge
rtk cargo test --locked --test project_smoke_browser
rtk cargo test --locked --test events_backfill
rtk node scripts/generated-frontend-browser-smoke.mjs
```

If a tool is missing, write the missing dependency as an environment gate and continue with lower tiers that remain meaningful.

## Live/Manual Lab

Use this tier only with explicit live approval or when the user asks for real Stellar CLI, Docker, local network, testnet, or pubnet validation.

Minimum sequence:

1. Probe `stellar --version`, plugin discovery, Docker, and network access.
2. Build `stellar-forge`.
3. Create a temp workspace.
4. Run `doctor`, `project validate`, and dry-run release planning.
5. Only then run Docker/local-network or testnet commands.
6. Redact secrets and account-specific output in evidence.

Live failures must be classified as product, Stellar CLI, Docker/local-network, RPC/network, account funding, or environment.

## Evidence Format

Use this structure in `_workspace/test-harness/05_heavy_integration_lab.md`:

```markdown
# Heavy Integration Lab

## Scope

## Environment Probe

## Tier Matrix

## Commands Run

## Findings

## Product Bugs

## Environment Gates

## Next Wave
```

## Test Scenarios

Normal flow: the user asks for heavy integration confidence. The lab probes tools, runs PR-safe gates, expands into heavy local generated-stack tests, and reports live-only gaps separately.

Error flow: a browser, Docker, `stellar`, or `sqlite3` command is missing or flaky. The lab keeps deterministic tests honest, marks the missing tool as an environment gate, and does not claim live coverage.
