# AGENTS.MD

## Project Snapshot

- `stellar-forge` is a Rust CLI for manifest-driven Stellar workspaces.
- The CLI owns scaffolding, manifest parsing, validation, dry runs, lockfile updates, release planning, and generated helper files.
- Chain-facing behavior is delegated to the official `stellar` CLI, so local environment differences matter during verification.

## Read First

- `README.md` for project overview, install, and common workflows
- `docs/command-reference.md` for the exact CLI surface
- `docs/manifest-reference.md` for `stellarforge.toml`, `stellarforge.lock.json`, and generated files
- `docs/deployment-guide.md` for release and deployment flows

## Repo Map

- `src/cli.rs`: Clap definitions for the full command surface and global flags
- `src/commands.rs`: command dispatcher plus shared helpers for report persistence and init flow
- `src/commands/`: feature-specific command implementations such as `doctor`, `events`, `release`, `token`, and `wallet`
- `src/model.rs`: manifest, lockfile, and config model types
- `src/runtime.rs`: `AppContext`, command execution helpers, and report rendering
- `src/templates.rs`: generated project scaffolds and helper script templates
- `tests/`: integration-style CLI coverage built around `assert_cmd`

## Working Rules

- Keep behavior, documentation, templates, and tests aligned in the same change.
- Preserve JSON output contracts unless the change explicitly updates that contract.
- Prefer focused integration tests for user-visible behavior over broad refactors.
- Follow existing command/reporting patterns before introducing a new abstraction.
- Treat `src/templates.rs` as user-facing output: changes there usually need test assertion updates and documentation updates.

## Local Dev Loop

```bash
cargo build
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --locked
```

Useful narrower loops:

```bash
cargo test --locked --test cli
cargo test --locked --test release_commands
cargo test --locked --test wallet_policy
```

## External Tooling Notes

- `stellar` CLI: required for many chain-facing flows and some scaffold generation paths
- Node.js plus a package manager: required for generated API/frontend projects
- Docker: required for local network workflows
- `sqlite3`: required for persisted event backfill and cursor reset flows

If a command or test fails because one of these is missing, call that out explicitly instead of masking it.

## Change Guidance

When touching CLI behavior:

- update the command definitions in `src/cli.rs` when flags or subcommands change
- update the relevant command implementation in `src/commands.rs` or `src/commands/*.rs`
- update docs in `README.md` or `docs/command-reference.md`
- add or adjust integration coverage in `tests/`

When touching generated project output:

- update `src/templates.rs`
- update assertions in `tests/cli.rs` or the most specific scaffold test
- update user-facing docs if scaffolded commands, files, or workflows change

When touching manifest or release behavior:

- prefer targeted tests around lockfile/report output
- keep `--json` output stable and easy to diff
- verify dry-run paths where possible because they are part of the UX contract

## Verification Expectations

- Run the narrowest useful tests while iterating.
- Before handing off shared or risky changes, run at least `cargo fmt --all` and relevant tests.
- Prefer full `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --locked` before finishing when the change affects shared command, model, runtime, or template behavior.

