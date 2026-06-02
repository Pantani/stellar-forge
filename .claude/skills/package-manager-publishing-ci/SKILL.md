---
name: package-manager-publishing-ci
description: Use when stellar-forge work mentions package-manager publishing, crates.io, cargo publish, Homebrew tap, brew formula, snapcraft, Snap Store, Debian packages, APT repositories, .deb artifacts, package signing, or release CI publishing.
---

# Package Manager Publishing CI

## Core Principle

Each package manager is its own release channel with different ownership, credentials, idempotence, and review rules. CI should publish channels independently after a shared preflight proves the release.

## Recommended CI Shape

1. Trigger on `push` tags matching `v*.*.*` and `workflow_dispatch` with an existing tag.
2. Run a `preflight` job that validates version, metadata, changelog, release assets, and required secrets.
3. Publish channels as independent jobs depending on `preflight`.
4. Use GitHub Environments for real publish credentials.
5. Upload generated package artifacts even when registry upload is blocked, so maintainers can inspect them.

## Channel Playbooks

| Channel | CI job shape | Required setup |
| --- | --- | --- |
| crates.io | `cargo publish --locked` after dry-run | `CARGO_REGISTRY_TOKEN`, Cargo metadata, package ownership |
| Homebrew tap | update formula in tap repo and open/push PR | tap repo, token or app permission, archive URL, SHA-256 |
| `.deb` artifact | build package and upload to GitHub release | package metadata and reproducible packaging command |
| APT repo | sign packages, update repository indexes, upload | signing key, repository host, stable distribution path |
| Snap Store | build snap and upload/release | snap name ownership, `SNAPCRAFT_STORE_CREDENTIALS`, confinement approval |

## Idempotence Rules

- A duplicate version can be success only if the remote artifact matches the current release.
- A failed channel must not roll back successful channels automatically; report partial publish state clearly.
- Reruns should skip channels already published for the same version when the registry supports detection.
- Never overwrite release artifacts for an existing tag unless the user explicitly requests a repair release and the registry permits it.

## Workflow Guardrails

- Do not expose publish secrets to pull requests.
- Do not run real publish steps when the event tag is missing, malformed, or does not match `Cargo.toml`.
- Do not publish APT or snap from a local workstation fallback when CI secrets are missing.
- Keep dry-run commands near real publish commands so failures are reproducible locally.

## Useful Local Checks

```bash
rtk cargo package --locked
rtk cargo publish --dry-run --locked
rtk gh release view vX.Y.Z --repo Pantani/stellar-forge --json assets
```
