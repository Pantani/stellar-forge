---
name: rust-cli-contract-testing
description: Use for stellar-forge Rust CLI unit tests, assert_cmd integration tests, command dispatch tests, manifest validation tests, runtime helper tests, report persistence tests, and deterministic dry-run behavior. Trigger on unit, integration, contract, CLI, --json, --out, manifest, runtime, command, or dry-run test work.
---

# Rust CLI Contract Testing

## Workflow

1. Read the command definition in `src/cli.rs`.
2. Read the command implementation and neighboring tests.
3. Choose the smallest test layer:
   - Unit test for pure helper logic.
   - Integration test in `tests/` for command behavior.
   - Docs contract test when examples or command references are involved.
4. Write the failing test first for behavior changes.
5. Patch implementation only as far as needed.
6. Run the focused cargo command.

## Test Style

- Use `assert_cmd` for CLI integration.
- Use `tempfile` for isolated workspaces.
- Use `serde_json::Value` for JSON report contracts.
- Assert action, status, key data fields, commands, artifacts, and exit code.
- Avoid large snapshots unless the artifact is intentionally snapshot-like.

## Offline Defaults

Use fake binaries in `PATH` for `stellar`, package managers, `git`, or `cargo` when the behavior is about orchestration. Use `--dry-run` whenever side effects are not the point of the test.

## Verification

Typical focused commands:

```bash
rtk cargo test --locked --lib <test_name>
rtk cargo test --locked --test cli <test_name>
rtk cargo test --locked --test release_commands <test_name>
```
