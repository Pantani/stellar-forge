---
name: ci-offline-quality-gates
description: Use for stellar-forge CI workflow tests, cargo quality gates, security audit gates, coverage command design, offline-vs-live split, flake triage, and external dependency classification. Trigger on CI, GitHub Actions, cargo fmt, clippy, cargo test, cargo audit, cargo deny, CodeQL, Playwright, Docker, stellar CLI, sqlite3, flake, or live smoke.
---

# CI Offline Quality Gates

## Gate Tiers

- PR: `cargo fmt --all --check`, `cargo check --locked --all-targets --all-features`, `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`, `cargo test --locked`, and deterministic offline smokes.
- Scheduled: broader browser smoke, security audits, dependency health.
- Manual/live: real Stellar CLI network, Docker quickstart, cold Playwright/browser installs, and long-running release drills.

## Workflow

1. Inspect current workflows and local commands.
2. Decide which tier the new check belongs to.
3. Keep PR checks deterministic unless the user explicitly accepts live dependencies.
4. Add local reproduction commands alongside workflow changes.
5. Document missing-tool behavior.

## Verification

Before promoting a gate, run the local equivalent. If unavailable, report the exact missing dependency and keep the workflow change separate from product fixes.
