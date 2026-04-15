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

