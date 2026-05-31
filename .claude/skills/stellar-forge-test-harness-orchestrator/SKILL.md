---
name: stellar-forge-test-harness-orchestrator
description: Use whenever stellar-forge work mentions tests, coverage, harness, unit tests, contract tests, integration tests, e2e, smoke, CI, flake, deterministic, idempotent, resilient, dry-run, generated scaffold drift, JSON contracts, rerun, update, fix, improve, or partial re-run. This skill must orchestrate the specialist test team and preserve offline deterministic defaults.
---

# Stellar Forge Test Harness Orchestrator

## Purpose

Coordinate repo-wide test and resilience work for `stellar-forge`, a Rust CLI that generates Stellar workspaces and delegates chain-facing behavior to the official `stellar` CLI.

## Phase 0: Context Check

1. Read `AGENTS.MD`, `CLAUDE.MD`, `README.md`, and relevant docs.
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

## Data Flow

Write intermediate notes under `_workspace/test-harness/`:

- `00_inventory.md`
- `01_gap_matrix.md`
- `02_wave_plan.md`
- `json-contracts.md`
- `generated-workspace.md`
- `idempotence.md`
- `ci-gates.md`

Keep final user-facing changes in code, tests, docs, or CI. Do not rely on `_workspace` as the only deliverable.

## Wave Strategy

1. Baseline inventory: map current tests, commands, docs, and CI gates.
2. P0 deterministic coverage: unit tests for pure helpers, JSON contract tests, dry-run purity, template matrix, idempotent sync.
3. Offline E2E: temp workspaces with fake `stellar`, package managers, HTTP loopback, and no live network.
4. Generated output drift: demo and scaffold parity, stale file policy, broad doctor/project checks.
5. Optional/live gates: browser, Docker, real Stellar CLI, and network smoke tests only when explicitly tiered.

## Error Handling

- Reproduce failures with the narrowest command.
- Identify the failing boundary before patching.
- If an external tool is missing, report it as a tooling gate and keep offline tests honest.
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

Error flow: a test fails because `stellar`, Node, Docker, Playwright, or `sqlite3` is unavailable. The orchestrator classifies the dependency and either fakes it for offline PR coverage or moves it to a live/manual gate.
