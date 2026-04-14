# Contributing to stellar-forge

Thanks for helping improve `stellar-forge`.
This project is a Rust CLI with a large amount of user-facing generated output, so the most useful
contributions keep behavior, docs, and tests aligned.

## Before you start

- search existing issues and pull requests before starting new work
- prefer opening an issue first for larger features, behavior changes, or workflow additions
- keep changes scoped to one problem when possible

## Local setup

Install the Rust stable toolchain and clone the repository.
From the repository root, the usual loop is:

```bash
cargo build
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --locked
cargo audit
```

Some commands and generated fixtures assume the official `stellar` CLI exists on `PATH`.
The test suite already handles the absence of that dependency for many dry-run flows, but you
should mention missing external tools when reporting failures.

## What to include in a change

When behavior changes, please update the same surfaces a user would notice:

- CLI tests in `tests/cli.rs` for command behavior and generated files
- documentation in `README.md` or `docs/` for new commands, flags, or workflows
- template output in `src/templates.rs` when generated scaffolds change

Please avoid unrelated refactors in the same pull request.
Small, reviewable changes are easier to land and easier to revert if a regression slips through.

## Coding guidelines

- follow the existing command and reporting patterns already present in `src/`
- keep JSON output stable unless the change explicitly updates the contract
- prefer explicit tests for user-visible behavior instead of relying only on manual verification
- document new external dependencies or system requirements in `README.md`
- run `cargo fmt`, `cargo clippy`, and `cargo test` before opening a pull request

## Pull requests

Pull requests are easiest to review when they include:

- a short explanation of the problem
- the concrete behavior change
- the validation commands you ran
- follow-up work or trade-offs that reviewers should know about

The repository PR template mirrors this checklist.

## Reporting bugs and asking for help

- bugs: use the GitHub bug report template
- feature ideas: use the feature request template
- usage questions: use the support template
- security issues: follow [SECURITY.md](SECURITY.md) and avoid public disclosure

## Community standards

Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before participating.
We want reviews, issue triage, and design discussions to stay respectful, specific, and useful.
