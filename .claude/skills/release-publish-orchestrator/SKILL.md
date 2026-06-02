---
name: release-publish-orchestrator
description: Use whenever stellar-forge work mentions publish, publishing, package managers, release CI, new version, tags, crates.io, cargo publish, Homebrew, brew tap, snap, snapcraft, Debian, APT, .deb, rerun, update, fix, improve, or partial re-run of publish automation.
---

# Stellar Forge Release Publish Orchestrator

## Purpose

Coordinate the publish team that prepares and maintains CI-driven distribution for `stellar-forge` when a new version is tagged or released.

## Phase 0: Context Check

1. Read `AGENTS.MD`, `CLAUDE.MD`, `README.md`, `CHANGELOG.md`, `Cargo.toml`, and `.github/workflows/release.yml`.
2. Inspect `.claude/agents/`, `.claude/skills/`, `.github/workflows/`, current git status, and `_workspace/publish-harness/` if present.
3. Decide execution mode:
   - Initial run: create `_workspace/publish-harness/` and write the release-channel inventory.
   - Partial rerun: use prior artifacts and touch only requested channels.
   - New version: archive old notes under `_workspace/publish-harness/previous/` before regenerating.

## Execution Mode

Prefer an agent team when two or more channels are involved. In Claude Code, use the project agent files. In Codex, use multi-agent tools when available or keep the same role split manually with disjoint file scopes.

Core team:

- `release-artifact-readiness-auditor`
- `crates-homebrew-publisher`
- `linux-package-publisher`
- `publish-ci-security-auditor`

When invoking agents, use `model: "opus"`.

## Data Flow

Write intermediate notes under `_workspace/publish-harness/`:

- `00_release_readiness.md`
- `01_channel_matrix.md`
- `crates_homebrew.md`
- `linux_packages.md`
- `ci_security.md`
- `publish_plan.md`

Keep durable changes in `.github/workflows/`, package metadata, docs, or the harness files. Do not rely on `_workspace` as the only deliverable.

## Publish Strategy

1. Release readiness: verify tag, Cargo version, changelog, Cargo metadata, release archives, and checksums.
2. CI security: confirm triggers, permissions, secrets, environments, and dry-run preflight.
3. Rust/macOS channels: prepare crates.io and Homebrew tap publishing.
4. Linux channels: prepare `.deb`, APT repository, and snap publishing.
5. Final verification: run local dry-runs/static checks and summarize per-channel go/no-go.

## Channel Policy

- crates.io and direct GitHub release archives are the first automation targets.
- Homebrew tap is preferred before attempting Homebrew core.
- `.deb` artifacts can ship before a full APT repository exists.
- APT publishing requires signing and a repository host.
- Snap publishing requires package-name ownership, confinement decision, and store credentials.

## Error Handling

- If metadata is missing, block all real publish jobs and report the missing fields.
- If one channel lacks secrets, keep other channels eligible and mark the blocked channel explicitly.
- If a registry already has the version, verify remote equality before treating rerun as safe.
- If three consecutive publish attempts fail, stop and re-audit channel readiness instead of patching blindly.

## Completion Criteria

Before claiming publish automation is ready, run the narrowest relevant dry-runs and static checks. For harness-only edits, at minimum run:

```bash
rtk git diff --check
rtk find .claude/agents .claude/skills -maxdepth 3 -type f
```

For real workflow or package metadata changes, also run:

```bash
rtk cargo package --locked
rtk cargo publish --dry-run --locked
```

If a registry or secret is unavailable, report the exact blocked channel and keep the workflow dry-run or disabled for that channel.

## Test Scenarios

Normal flow: user asks to publish a new version. The orchestrator validates the tag and release assets, checks secrets, updates channel jobs, runs dry-runs, and reports the channel matrix.

Error flow: crates.io is ready but Snap Store credentials are missing. The orchestrator keeps crates.io eligible, marks snap blocked with the missing secret, and avoids exposing any token values.
