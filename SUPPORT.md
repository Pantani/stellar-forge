# Support

This document explains where to start when something is unclear, broken, or behaving differently
than you expected.

## Start with the docs

Most usage questions are answered in the docs that ship with the repository:

- [README.md](README.md)
- [docs/README.md](docs/README.md)
- [docs/command-reference.md](docs/command-reference.md)
- [docs/manifest-reference.md](docs/manifest-reference.md)
- [docs/deployment-guide.md](docs/deployment-guide.md)

Good shortcuts by problem:

| Problem | Best starting doc |
| --- | --- |
| "How do I run this command?" | [docs/command-reference.md](docs/command-reference.md) |
| "What should go in `stellarforge.toml`?" | [docs/manifest-reference.md](docs/manifest-reference.md) |
| "Why did my release or rollback behave this way?" | [docs/deployment-guide.md](docs/deployment-guide.md) |
| "What files should I expect to change?" | [docs/manifest-reference.md](docs/manifest-reference.md#which-commands-update-which-files) |
| "How do I bootstrap a project quickly?" | [README.md](README.md#quick-start) |

## Which template to use

When you need maintainer help, choose the closest issue or discussion path:

- use the bug report template for reproducible defects
- use the feature request template for workflow improvements
- use the support question template for setup, usage, or troubleshooting help
- use [SECURITY.md](SECURITY.md) for vulnerabilities and avoid public disclosure

## Before opening a support request

Please try these first:

1. run `stellar forge doctor`
2. run `stellar forge project validate`
3. rerun the failing command with `--json` or `--out`
4. confirm whether the official `stellar` CLI is installed and visible on `PATH`
5. check whether the issue is specific to one network, one manifest, or one machine

That short loop answers a surprising number of "is it my setup or the command?" questions.

## Information that helps a lot

Include as much of this as you can:

- the exact command you ran
- the full error output
- whether you used `--dry-run`, `--json`, or `--out`
- your operating system
- your Rust version
- whether `stellar`, Docker, Node.js, package manager tooling, and `sqlite3` are installed
- whether the problem happens in a fresh scaffold or only in an existing workspace
- the relevant snippet of `stellarforge.toml` when config matters

Best possible support report:

```text
Command
stellar forge --json release verify testnet --out dist/release.verify.json

Environment
- macOS 15
- rustc 1.xx.x
- stellar CLI installed
- Docker installed

Observed
- report status is warn
- warning mentions deploy artifact drift

Expected
- verify should be clean after env export
```

## Common troubleshooting checklist

### Command fails immediately

Check:

- does `stellar forge doctor` pass?
- is the command being run from the workspace root?
- does `stellarforge.toml` exist where you think it does?
- are you accidentally pointing at a different manifest with `--manifest` or `--cwd`?

### Chain-facing command fails

Check:

- is the official `stellar` CLI installed?
- is the target identity available through the Stellar CLI?
- are you using the right network?
- does `--dry-run` succeed?

### Local dev flow fails

Check:

- does `[networks.local]` exist with `kind = "local"`?
- is Docker available?
- did `dev up` or `dev reseed` regenerate `.env.generated` as expected?

### Event commands fail

Check:

- is `sqlite3` available?
- was event ingestion initialized?
- does `workers/events/cursors.json` exist?
- are you backfilling a supported resource kind such as contract, token, or account?

### Generated API or frontend looks stale

Check:

- run `stellar forge project sync`
- run `stellar forge doctor fix --scope api`
- run `stellar forge doctor fix --scope frontend`
- rerun the relevant smoke command

### Release state looks inconsistent

Check:

- `stellar forge release status <env>`
- `stellar forge release drift <env>`
- `stellar forge release history <env>`
- `stellar forge release inspect <env>`

Those usually tell you whether the mismatch is in the manifest, the lockfile, the local artifacts,
or the live network.

## Useful commands to attach in support requests

These are often the most helpful reports to include:

```bash
stellar forge doctor --out dist/doctor.json
stellar forge project validate --out dist/project.validate.json
stellar forge project info --out dist/project.info.json
stellar forge release status testnet --out dist/release.status.json
stellar forge release drift testnet --out dist/release.drift.json
stellar forge events status --out dist/events.status.json
```

If the issue is with one command, attaching that command's report is even better.

## Asking for faster help

You are much more likely to get a quick, precise answer when you provide:

- a command that can be copied and rerun
- a minimal manifest or fixture
- one clear expected behavior
- one clear observed behavior

The more a maintainer has to infer, the slower and fuzzier the answer gets.
