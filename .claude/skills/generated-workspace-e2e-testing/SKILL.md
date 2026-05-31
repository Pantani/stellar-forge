---
name: generated-workspace-e2e-testing
description: Use for stellar-forge generated workspace tests, template matrix coverage, demo drift, project sync, doctor fix, API/frontend scaffold files, browser smoke, fake package managers, and offline E2E flows. Trigger on templates, scaffolds, generated files, demo, e2e, smoke, browser, project sync, or doctor fix.
---

# Generated Workspace E2E Testing

## Workflow

1. Read `src/templates.rs`, the relevant command implementation, and docs.
2. Identify whether the check is PR-safe offline or live/manual.
3. Build temp workspaces with fake tools unless live behavior is explicitly required.
4. Assert generated module shape and selected content.
5. Re-run generation to check content idempotence when relevant.

## Coverage Targets

- All documented init templates.
- API OpenAPI and manifest module outputs.
- Frontend generated state and smoke runners.
- Event worker/cursor files.
- Demo artifact drift when versioned generated files are intended to stay current.

## Boundaries

Do not call live Stellar network, Docker, or cold Playwright in default PR tests. Promote those to scheduled/manual gates through `ci-quality-gate-auditor`.

## Verification

Useful commands:

```bash
rtk cargo test --locked --test cli project_sync
rtk cargo test --locked --test cli init_template
rtk cargo test --locked --test project_smoke_browser
rtk cargo test --locked --test doctor_fix_scopes
```
