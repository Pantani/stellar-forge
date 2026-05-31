# Wave 03: Shared Test Support Helpers

## Scope

- Centralize repeated integration-test setup for generated demo workspaces.
- Centralize the offline fake `stellar` binary used by wallet and backlog coverage.
- Keep CLI behavior unchanged; this wave only reduces test setup drift.

## Changes

- Added `tests/support/mod.rs`.
- Migrated `tests/backlog_command_behaviors.rs`.
- Migrated `tests/wallet_policy.rs`.

## Verification

- Passed: `cargo fmt --all`.
- Passed: `cargo test --locked --test backlog_command_behaviors` (10 tests).
- Passed: `cargo test --locked --test wallet_policy` (7 tests).
- Passed: `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`.
- Passed: `cargo test --locked` (225 tests, 18 suites).
- Passed: `git diff --check`.
