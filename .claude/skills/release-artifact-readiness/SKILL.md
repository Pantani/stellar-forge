---
name: release-artifact-readiness
description: Use when stellar-forge publish work mentions release readiness, version tags, changelog, Cargo metadata, GitHub release assets, checksums, package provenance, duplicate versions, or go/no-go decisions before package-manager publishing.
---

# Release Artifact Readiness

## Core Principle

Package managers should consume a proven release, not create one implicitly. Verify the source tag, package metadata, release artifacts, and checksums before any publish job runs.

## Workflow

1. Read `Cargo.toml`, `Cargo.lock`, `README.md`, `CHANGELOG.md`, and `.github/workflows/release.yml`.
2. Verify the requested tag exists and matches `Cargo.toml` version without the `v` prefix.
3. Check that `CHANGELOG.md` has notes for the release version.
4. Check Cargo package metadata needed by registries: description, license or license-file, repository, homepage, readme, keywords, and categories.
5. Verify GitHub release archives and `.sha256` files exist for documented targets.
6. Produce a per-channel go/no-go matrix before touching registries.

## Channel Readiness

| Channel | Minimum readiness evidence |
| --- | --- |
| crates.io | `cargo package --locked` and `cargo publish --dry-run --locked` pass |
| Homebrew | source or binary archive URL plus SHA-256 is stable for the tag |
| `.deb` | package metadata, license, version, maintainer, and reproducible package command exist |
| APT | `.deb` exists plus signed repository index and upload target are configured |
| snap | `snapcraft.yaml`, package name ownership, confinement decision, and store credentials are ready |

## Common Mistakes

- Publishing from `main` instead of an immutable tag.
- Treating a `.deb` artifact as an APT repository.
- Assuming duplicate-version registry errors are harmless without comparing the existing artifact.
- Letting a missing secret fail halfway through a publish workflow instead of blocking in preflight.

## Verification Commands

```bash
rtk git rev-parse vX.Y.Z
rtk cargo package --locked
rtk cargo publish --dry-run --locked
rtk gh release view vX.Y.Z --repo Pantani/stellar-forge --json tagName,assets
```
