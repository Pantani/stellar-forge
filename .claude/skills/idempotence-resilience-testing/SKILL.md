---
name: idempotence-resilience-testing
description: Use for stellar-forge idempotence tests, atomic writes, repeated command behavior, dry-run purity, stale generated files, path safety, report persistence hardening, and resilience fixes. Trigger on idempotent, deterministic, resilient, atomic, dry-run, stale, drift, safe path, repeated run, or partial write.
---

# Idempotence Resilience Testing

## Workflow

1. Trace the stateful surface: input files, command, output files, reports, external tools.
2. Write a test that runs the operation twice or removes/corrupts a managed artifact.
3. Assert content stability, expected repair, or explicit warning.
4. Keep manual user-owned files outside managed output untouched.
5. Prefer atomic writes for managed files.

## Patterns

- Dry-run purity: command plans without requiring external tools and without writing files.
- Content idempotence: repeated sync produces identical generated contents.
- Repair idempotence: doctor/fix restores missing files and reports remaining drift.
- Safety: path traversal and unsafe names fail before file writes.

## Verification

Useful commands:

```bash
rtk cargo test --locked --lib safe
rtk cargo test --locked --test cli dry_run
rtk cargo test --locked --test cli idempotent
rtk cargo test --locked --test events_backfill
```
