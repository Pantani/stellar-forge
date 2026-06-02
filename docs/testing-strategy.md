# Testing Strategy

`stellar-forge` needs more than one kind of test because it sits between local project files and the official `stellar` CLI. A good test wave says which boundary it proves instead of flattening everything into one pass/fail result.

## Test Tiers

| Tier | What it proves | Where it belongs |
| --- | --- | --- |
| PR-safe offline | CLI contracts, manifest parsing, reports, dry-run behavior, generated-file content, fake external commands | Pull requests and normal local iteration |
| Heavy local | Generated API/frontend/event stack, real Node/package-manager/browser/SQLite tooling, repeated sync/fix/smoke flows | Local release confidence or scheduled CI |
| Live/manual | Official `stellar` CLI, plugin discovery, Docker/local network, testnet/pubnet, funded-account behavior | Manual or explicitly scheduled validation |

The project should not claim live Stellar confidence from offline tests, and it should not make routine PR checks depend on live network state.

## Default Local Gate

Use this when a change touches shared CLI behavior, runtime, model, templates, or report contracts:

```bash
cargo fmt --all --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked
```

Focused integration loops:

```bash
cargo test --locked --test cli_command_matrix
cargo test --locked --test generated_workspace_sync
cargo test --locked --test project_smoke_browser
cargo test --locked --test events_backfill
```

Most PR-safe tests should use temp workspaces, fake `stellar` binaries, fake package-manager calls, dry-run modes, or dependency skips when the behavior under test is orchestration rather than the external tool itself.

## Heavy Local Lab

Use this when the goal is high-confidence validation before a major handoff:

1. Probe Rust, `stellar`, Node.js, package manager, Docker, browser, and `sqlite3`.
2. Build the CLI with locked dependencies.
3. Generate temp workspaces from the templates relevant to the change.
4. Run `project validate`, `project sync`, `doctor fix`, and smoke paths.
5. Use real Node/package-manager/browser/SQLite tooling when present.
6. Repeat stateful commands and compare generated output for drift.
7. Keep live chain calls dry-run or fake unless the live tier is explicitly selected.

Useful commands:

```bash
cargo build --locked --bin stellar-forge
cargo test --locked --test project_smoke_browser
cargo test --locked --test events_backfill
node scripts/generated-frontend-browser-smoke.mjs
```

If a dependency is missing, record it as an environment gate and keep lower tiers honest.

## Live/Manual Smoke

Use this only when the user or release process explicitly asks for real Stellar CLI, Docker/local network, testnet, or pubnet validation.

Minimum order:

1. Probe `stellar --version`, plugin discovery, Docker, and network readiness.
2. Build `stellar-forge`.
3. Create a temp workspace.
4. Run `doctor`, `project validate`, and dry-run release planning.
5. Run the smallest approved Docker/local-network or testnet command sequence.
6. Redact secrets, account identifiers, and environment-specific paths in shared evidence.

Every live failure should be classified as product code, official Stellar CLI behavior, Docker/local-network setup, RPC/network availability, account funding, or local environment dependency before a fix is attempted.

## Harness Team

The durable harness lives under `.claude/`:

- `.claude/skills/stellar-forge-test-harness-orchestrator/SKILL.md`
- `.claude/skills/heavy-integration-test-lab/SKILL.md`
- `.claude/skills/live-stellar-smoke-testing/SKILL.md`
- `.claude/skills/generated-stack-stress-testing/SKILL.md`
- `.claude/agents/heavy-integration-lab-runner.md`
- `.claude/agents/stellar-cli-live-smoke-qa.md`
- `.claude/agents/generated-stack-stress-qa.md`

Use `_workspace/test-harness/` for evidence from large waves so later runs can resume from real findings instead of starting from memory.
