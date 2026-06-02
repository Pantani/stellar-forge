# CLAUDE.MD

This repository uses `AGENTS.MD` as the canonical agent playbook. Claude should follow that file in full.

## Quick Start

1. Read `AGENTS.MD`.
2. Read `README.md` and the relevant file under `docs/` before changing user-facing behavior.
3. Use the standard Rust verification loop:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --locked
```

## Harness: Stellar Forge Test Coverage

**Goal:** Expand deterministic unit, contract, integration, E2E, idempotence, and resilience coverage for the CLI and generated workspaces.

**Trigger:** For `stellar-forge` test coverage, harness, CI, contract tests, E2E, idempotence, dry-run purity, generated scaffold drift, or resilience work, use `.claude/skills/stellar-forge-test-harness-orchestrator/SKILL.md`. Simple questions can still be answered directly.

**Change History:**
| Date | Change | Target | Reason |
| --- | --- | --- | --- |
| 2026-05-31 | Added generated workspace sync dry-run and repair coverage | `tests/generated_workspace_sync.rs`, `tests/support` | Prove scaffold drift can be previewed safely and repaired idempotently |
| 2026-05-31 | Added shared integration-test support helpers for fake Stellar workspaces | `tests/support`, selected integration suites | Keep offline E2E coverage deterministic and reduce duplicated setup drift |
| 2026-05-31 | Added test harness team and orchestrator pointer | `.claude/agents`, `.claude/skills`, `CLAUDE.MD` | Repo-wide deterministic test coverage and resilience work |

## Harness: Stellar Forge Publish Automation

**Goal:** Prepare and maintain CI-driven publishing for release channels such as crates.io, Homebrew tap, Snap Store, `.deb` artifacts, and signed APT repositories.

**Trigger:** For `stellar-forge` release publishing, package-manager distribution, version-tag publish automation, crates.io, Homebrew, snap, Debian, APT, package signing, registry secrets, or release CI work, use `.claude/skills/release-publish-orchestrator/SKILL.md`. Simple package-manager questions can still be answered directly.

**Change History:**
| Date | Change | Target | Reason |
| --- | --- | --- | --- |
| 2026-06-02 | Aligned release publishing with Sunscreen PR #18 fan-out model | `.github/workflows/release.yml`, `docs/release-publishing.md`, `README.md` | Publish channels run best-effort after GitHub Release and report aggregate status without blocking core release binaries |
| 2026-06-02 | Added MIT license, Debian/Snap packaging metadata, and release artifact jobs | `Cargo.toml`, `LICENSE`, `snap/snapcraft.yaml`, `.github/workflows/release.yml`, `docs/release-publishing.md` | Unblock package-manager CI once the release tag and Homebrew tap exist |
| 2026-06-02 | Added release CI preflight and package-manager publishing preparation | `.github/workflows/release.yml`, `.github/scripts`, `docs/release-publishing.md` | Prepare safe CI-driven publishing while blocking external registries until metadata, secrets, and environments are ready |
| 2026-06-02 | Added release publish automation harness team and orchestrator pointer | `.claude/agents`, `.claude/skills`, `CLAUDE.MD` | Coordinate safe CI publishing across crates.io, Homebrew, snap, `.deb`, and APT channels |

## Repo-Specific Reminders

- `stellar-forge` is a Rust CLI that shells out to the official `stellar` CLI for network-facing work.
- Changes to `src/templates.rs` usually require test updates and doc updates.
- Preserve `--json` output structure unless you are intentionally changing the contract.
- Prefer focused integration tests in `tests/` for user-visible behavior.

## High-Value Files

- `src/cli.rs`
- `src/commands.rs`
- `src/commands/`
- `src/model.rs`
- `src/runtime.rs`
- `src/templates.rs`
- `tests/`
