# Wave 04: Generated Workspace Sync

## Scope

- Cover command-level dry-run purity for `project sync`.
- Cover recreation of removed API/frontend scaffold files.
- Confirm regenerated workspaces return to clean `project validate`.

## Changes

- Added `tests/generated_workspace_sync.rs`.
- Made shared `tests/support` helpers reusable across integration crates without per-crate dead-code drift.

## Verification

- Passed: `cargo fmt --all`.
- Passed: `cargo test --locked --test generated_workspace_sync` (2 tests).
- Passed: `cargo fmt --all --check`.
- Passed: `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`.
- Passed: `cargo test --locked` (227 tests, 19 suites).
- Passed: `git diff --check`.
