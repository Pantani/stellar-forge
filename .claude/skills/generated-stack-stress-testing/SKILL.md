---
name: generated-stack-stress-testing
description: Use for stellar-forge generated API/frontend/event-worker stress tests, real Node or package-manager validation, browser smoke reliability, SQLite-backed generated app checks, template matrix expansion, repeated project sync/doctor fix, and reruns or updates of generated-stack evidence.
---

# Generated Stack Stress Testing

## Purpose

Stress the generated stack as a user would experience it: generated API, frontend, event worker, helper scripts, OpenAPI, generated frontend state, and smoke runners. This skill lives between normal offline integration tests and live Stellar network tests.

## Workflow

1. Read `src/templates.rs`, relevant command implementations, `README.md`, and docs.
2. Select templates and generated modules for the matrix.
3. Build temp workspaces; do not mutate user-owned project files.
4. Run scaffold validation and sync/fix loops.
5. Use real Node/package-manager/browser/SQLite tools when present.
6. Repeat stateful commands to detect generated drift.
7. Write evidence to `_workspace/test-harness/generated-stack-stress.md`.

## Matrix Axes

- template: `minimal-contract`, `fullstack`, `rewards-loyalty`, `api-only`, `multi-contract`
- module: contracts, API, frontend, event worker, scripts, release artifacts
- command: `project validate`, `project sync`, `project smoke`, `doctor fix`
- tool mode: fake, real local, live delegated
- state: fresh, drifted, partially deleted, repeated run

## Useful Commands

```bash
rtk cargo test --locked --test generated_workspace_sync
rtk cargo test --locked --test doctor_fix_scopes
rtk cargo test --locked --test project_smoke_browser
rtk node scripts/generated-frontend-browser-smoke.mjs
```

When `sqlite3` is present, include event-worker coverage:

```bash
rtk cargo test --locked --test events_backfill
```

## Boundaries

Do not call live Stellar network from this skill. If generated output needs real chain state, hand the live boundary to `live-stellar-smoke-testing` and keep this skill focused on generated files, app tooling, and local smoke.

## Test Scenarios

Normal flow: generated frontend changes. Run the focused Rust test, run the browser smoke runner with real local tooling if available, repeat `project sync`, and record any drift.

Error flow: Node or browser tooling is unavailable. Record the missing dependency, keep fake-tool and content assertions, and recommend a live/heavy rerun only after the tool is installed.
