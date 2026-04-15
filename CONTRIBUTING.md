# Contributing to stellar-forge

Thanks for helping improve `stellar-forge`.

This project is a Rust CLI, but a meaningful part of the user experience lives in generated output,
documentation, and JSON reports. The best contributions keep those surfaces aligned instead of only
changing the Rust implementation underneath them.

## Before you start

Good first moves:

1. search existing issues, pull requests, and docs before starting new work
2. open or comment on an issue first for larger features, behavior changes, or new workflows
3. keep the first version of a change scoped to one problem

That usually means:

- one command family or one behavior change per pull request
- one documentation thread per behavior change
- one reviewable test story instead of a broad cleanup

## What kind of project this is

`stellar-forge` owns:

- workspace scaffolding
- manifest parsing and validation
- dry-run planning and JSON reports
- lockfile updates and release artifacts
- generated API, frontend, relayer, and event-worker files

It does not reimplement the full Stellar toolchain. Many chain-facing flows shell out to the
official `stellar` CLI, so local environment differences matter during development and review.

## Read this first

If you are touching user-facing behavior, skim these before you code:

- [README.md](README.md)
- [docs/command-reference.md](docs/command-reference.md)
- [docs/manifest-reference.md](docs/manifest-reference.md)
- [docs/deployment-guide.md](docs/deployment-guide.md)

If you are touching generated project output, also read:

- [src/templates.rs](src/templates.rs)
- [demo/README.md](demo/README.md)

## Repo map

These files are the fastest way to orient yourself:

| Path | Why it matters |
| --- | --- |
| `src/cli.rs` | Clap command surface, flags, and subcommands |
| `src/commands.rs` | dispatch and shared command helpers |
| `src/commands/*.rs` | feature-specific implementations |
| `src/model.rs` | manifest, lockfile, scenario, and validation model |
| `src/runtime.rs` | app context, command execution, reports, and rendering |
| `src/templates.rs` | generated project output and helper scripts |
| `tests/cli.rs` | broad integration-style CLI coverage |
| `tests/*.rs` | focused coverage for newer command families and workflows |

## Local setup

Install the Rust stable toolchain and clone the repository.

From the repository root, the normal local loop is:

```bash
cargo build
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --locked
cargo audit
```

CI currently runs:

```bash
cargo fmt --all --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked
node scripts/generated-frontend-browser-smoke.mjs
cargo audit
```

## External tooling you may need

Some flows depend on tools outside Rust:

| Tool | Needed for |
| --- | --- |
| `stellar` CLI | most chain-facing contract, wallet, token, release, and event flows |
| Node.js | generated API and frontend scaffolds |
| package manager such as `pnpm` | generated apps and smoke flows |
| Docker | local network flows such as `dev up` |
| `sqlite3` | persisted event backfill and cursor workflows |

The test suite already handles the absence of some dependencies in dry-run paths, but if a command
or test fails because a tool is missing, call that out clearly instead of working around it
silently.

## Choosing the right test loop

You do not need the whole suite on every edit. Prefer the narrowest useful loop while iterating.

Useful patterns:

```bash
cargo test --locked --test cli
cargo test --locked --test release_commands
cargo test --locked --test wallet_policy
cargo test --locked --test events_backfill
cargo test --locked --test planned_commands_docs
```

Suggested rule of thumb:

- single command behavior change: run the most specific test file plus `cargo fmt`
- shared model, runtime, template, or report changes: run `cargo fmt`, targeted tests, and
  preferably `cargo clippy`
- generated frontend or scaffold changes: also consider the browser smoke path

## What to update for each kind of change

### CLI behavior

When flags, subcommands, or command semantics change, update together:

- `src/cli.rs`
- the relevant implementation in `src/commands.rs` or `src/commands/*.rs`
- docs in `README.md` or `docs/command-reference.md`
- focused tests in `tests/`

### Generated project output

When scaffolded files or helper scripts change, update together:

- `src/templates.rs`
- `demo/README.md` when the generated README meaningfully changes
- assertions in `tests/cli.rs` or the most specific scaffold-related test
- docs when the generated workflow changed from a user perspective

### Manifest or release behavior

When `stellarforge.toml`, lockfile semantics, or release artifacts change, update together:

- `src/model.rs` and any validation logic
- release docs in [docs/deployment-guide.md](docs/deployment-guide.md)
- config docs in [docs/manifest-reference.md](docs/manifest-reference.md)
- tests around lockfile, release reports, diff/drift/history, or rollback behavior

## Coding guidelines

- follow the existing command and report patterns already present in `src/`
- keep JSON output stable unless the change is explicitly updating the contract
- prefer explicit tests for user-visible behavior instead of relying only on manual verification
- document new external dependencies or system requirements in `README.md`
- keep generated output readable; it is part of the product
- avoid unrelated refactors in the same pull request

## Documentation expectations

Docs are not an afterthought here. Update them in the same pull request when behavior changes.

Usually that means:

- `README.md` for quick-start and high-level workflows
- `docs/command-reference.md` for exact syntax and examples
- `docs/manifest-reference.md` for config model or artifact changes
- `docs/deployment-guide.md` for release and recovery behavior

If your change introduces a new input file shape, add an example.
If your change introduces a new report artifact, document where it is written and how it is used.

## Pull requests

Pull requests are easiest to review when they include:

- a short explanation of the problem
- the concrete behavior change
- the exact validation commands you ran
- screenshots or copied report snippets when the change is primarily user-facing
- follow-up work or trade-offs reviewers should know about

A strong PR description often looks like this:

```text
Summary
- add release drift reporting for archived snapshots

Validation
- cargo fmt --all
- cargo test --locked --test release_commands --test release_status_diff

Notes
- docs updated in README and deployment guide
```

## Commit and review hygiene

- keep commits focused enough that a reviewer can understand them in one pass
- avoid mixing renames, refactors, and behavior changes unless they are tightly connected
- mention when a diff includes generated output on purpose
- call out any behavior that still depends on local environment setup

## Reporting bugs and asking for help

- bugs: use the GitHub bug report template
- feature ideas: use the feature request template
- usage questions: use the support template
- security issues: follow [SECURITY.md](SECURITY.md) and avoid public disclosure

When reporting a bug, include:

- the exact command you ran
- the current working directory or manifest path if it matters
- whether `stellar`, Docker, Node.js, and `sqlite3` are available locally
- the relevant report file when you ran with `--json` or `--out`

## Community standards

Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before participating.
We want reviews, issue triage, and design discussions to stay respectful, specific, and useful.
