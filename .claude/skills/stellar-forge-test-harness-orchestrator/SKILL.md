---
name: stellar-forge-test-harness-orchestrator
description: Use whenever stellar-forge work mentions tests, coverage, harness, unit tests, contract tests, integration tests, heavy integration, "testes de verdade", e2e, smoke, CI, flake, real tooling, live Stellar CLI, Docker, local network, testnet, deterministic, idempotent, resilient, dry-run, generated scaffold drift, JSON contracts, rerun, update, fix, improve, or partial re-run. This skill must orchestrate the specialist test team, tier heavy/live checks clearly, and preserve offline deterministic defaults.
---

# Stellar Forge Test Harness Orchestrator

## Purpose

Coordinate repo-wide test and resilience work for `stellar-forge`, a Rust CLI that generates Stellar workspaces and delegates chain-facing behavior to the official `stellar` CLI. The orchestrator separates normal deterministic PR checks from heavy local integration labs and explicit live/manual Stellar gates.

## Phase 0: Context Check

1. Read `AGENTS.MD`, `CLAUDE.MD`, `CONTRIBUTING.md`, `README.md`, and relevant docs.
2. Inspect `.claude/agents/`, `.claude/skills/`, current git status, and `_workspace/test-harness/` if present.
3. Decide execution mode:
   - Initial run: build a wave plan and create `_workspace/test-harness/`.
   - Partial rerun: use prior artifacts and touch only requested slices.
   - New input: archive old artifacts under `_workspace/test-harness/previous/` before regenerating.

## Execution Mode

Prefer an agent team when at least two specialists are useful. In Claude Code, use the project agent files. In Codex, use `multi_agent` subagents when explicitly authorized by the user or by a harness request, and keep write scopes disjoint.

Core team:

- `rust-cli-test-architect`
- `json-contract-qa`
- `generated-workspace-e2e`
- `idempotence-resilience-auditor`
- `ci-quality-gate-auditor`
- `heavy-integration-lab-runner`
- `generated-stack-stress-qa`
- `stellar-cli-live-smoke-qa`

Use `heavy-integration-lab-runner`, `generated-stack-stress-qa`, and `stellar-cli-live-smoke-qa` when the request asks for real confidence, heavy integration, generated app validation, live tooling, Docker/local network, or testnet checks.

## Data Flow

Write intermediate notes under `_workspace/test-harness/`:

- `00_inventory.md`
- `01_gap_matrix.md`
- `02_wave_plan.md`
- `json-contracts.md`
- `generated-workspace.md`
- `idempotence.md`
- `ci-gates.md`
- `05_heavy_integration_lab.md`
- `generated-stack-stress.md`
- `live-stellar-smoke.md`

Keep final user-facing changes in code, tests, docs, or CI. Do not rely on `_workspace` as the only deliverable.

## Wave Strategy

1. Baseline inventory: map current tests, commands, docs, and CI gates.
2. P0 deterministic coverage: unit tests for pure helpers, JSON contract tests, dry-run purity, template matrix, idempotent sync.
3. Offline E2E: temp workspaces with fake `stellar`, package managers, HTTP loopback, and no live network.
4. Generated output drift: demo and scaffold parity, stale file policy, broad doctor/project checks.
5. Heavy local integration: real Node/package-manager/browser/SQLite generated-stack checks in temp workspaces, with missing tools recorded as environment gates.
6. Real Stellar CLI boundary: plugin discovery, doctor/project validation, release dry-run planning, and command handoff checks using the installed official `stellar` CLI.
7. Optional live/manual gates: Docker/local network, testnet, pubnet, funded-account, and long release drills only when explicitly tiered.

## Test Tier Contract

Report each tier separately:

- PR-safe offline: deterministic Rust tests, fake binaries, dry runs, temp workspaces, and no live network.
- Heavy local: real generated stack, package manager, browser, SQLite, and broad matrices; still avoids funded live network by default.
- Live/manual: official `stellar` CLI, Docker/local network, testnet/pubnet, and account-dependent flows with explicit opt-in.

Do not claim "everything works" from only one tier. Say which tier passed, which tier was skipped, and why.

## Error Handling

- Reproduce failures with the narrowest command.
- Identify the failing boundary before patching.
- If an external tool is missing, report it as a tooling gate and keep offline tests honest.
- If a heavy or live test fails, classify the boundary first: product code, JSON/report contract, generated artifact, fake/real external command, Node/package manager, browser, SQLite, Docker/local network, Stellar CLI, RPC/network, or account funding.
- If three consecutive fixes fail, stop and reconsider the harness slice rather than layering patches.

## Completion Criteria

Before claiming done, run the narrowest relevant tests plus `cargo fmt --all`. For shared command, model, runtime, or template changes, prefer:

```bash
rtk cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
rtk cargo test --locked
```

If a full gate is too slow or blocked by missing external tooling, say exactly what ran and what remains.

## Test Scenarios

Normal flow: user asks to improve test coverage. The orchestrator inventories tests, picks one deterministic wave, dispatches specialists, integrates patches, and verifies with focused cargo commands.

Heavy flow: user asks for "testes de verdade". The orchestrator probes tools, runs PR-safe gates, expands into heavy local generated-stack tests, then records live-only Stellar/Docker/testnet gates separately.

Error flow: a test fails because `stellar`, Node, Docker, Playwright, or `sqlite3` is unavailable. The orchestrator classifies the dependency and either fakes it for offline PR coverage or moves it to a live/manual gate.
