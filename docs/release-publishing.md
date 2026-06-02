# Release Publishing

This project publishes release binaries first, then prepares package-manager channels from the same
tagged release.

## Current Release Flow

`.github/workflows/release.yml` runs on:

- `push` tags matching `v*.*.*`
- manual `workflow_dispatch` with an existing tag

The automatic tag flow builds Linux x86_64 and macOS x86_64/aarch64 archives, publishes them to the
GitHub Release, runs package-manager preflight, and attaches a generated `stellar-forge.rb`
Homebrew formula artifact to the release.

External package-manager publishing is intentionally gated. It only runs from manual
`workflow_dispatch` when:

1. `publish_external` is true
2. `confirm` exactly matches `PUBLISH vMAJOR.MINOR.PATCH`
3. the relevant protected GitHub Environment allows the job
4. the channel-specific preflight is ready

This lets a new tag create inspectable release assets without silently pushing to external
registries.

## Preflight Checks

The publish preflight validates:

- release tag format is `vMAJOR.MINOR.PATCH`
- `Cargo.toml` version matches the tag without the `v`
- `CHANGELOG.md` has a section for the version
- expected `.tar.gz` archives and `.sha256` files exist
- package-manager credentials and channel setup are present when needed

The repo currently has package metadata for description, repository, homepage, readme, keywords, and
categories. A real package-manager publish remains blocked until the project declares a `license` or
`license-file` and the repository includes the matching license text.

## Channels

| Channel | Current CI behavior | Required setup before real publish |
| --- | --- | --- |
| GitHub Release binaries | automatic on tag | tag points at the intended commit and CI is green |
| crates.io | manual, gated, currently blocked by license metadata | `license` or `license-file`, package ownership, `CARGO_REGISTRY_TOKEN` or trusted publishing setup |
| Homebrew formula artifact | automatic after GitHub Release | none for generated artifact |
| Homebrew tap PR | manual, gated, currently blocked by missing tap/license | create `Pantani/homebrew-tap` or set `HOMEBREW_TAP_REPO`, configure `HOMEBREW_TAP_TOKEN`, declare package license |
| `.deb` artifact | readiness summary only | license metadata, maintainer/package config, and a reproducible `cargo-deb` job |
| APT repository | blocked | signed repository host plus APT signing/upload secrets |
| Snap Store | blocked | `snap/snapcraft.yaml`, snap name ownership, confinement decision, and `SNAPCRAFT_STORE_CREDENTIALS` |

## Recommended GitHub Environments

Create these environments with required reviewers and prevent self-review where appropriate:

- `github-release`
- `crates-io`
- `homebrew-tap`
- `deb-artifacts`
- `apt-repo`
- `snap-store-candidate`
- `snap-store-stable`

Keep PR workflows read-only and never expose publish credentials to `pull_request` events.

## Secrets and Variables

| Name | Type | Used by |
| --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | environment secret | crates.io fallback token publish |
| `HOMEBREW_TAP_REPO` | repository or environment variable | override the default `Pantani/homebrew-tap` |
| `HOMEBREW_TAP_TOKEN` | environment secret | pushing formula branch and opening a tap PR |
| `SNAPCRAFT_STORE_CREDENTIALS` | environment secret | Snap Store upload/release |
| `APT_SIGNING_KEY` | environment secret | signed APT repository metadata |
| `APT_SIGNING_KEY_ID` | environment secret | selecting the APT signing identity |
| `APT_REPO_UPLOAD_TOKEN` | environment secret | upload to the chosen APT repository host |

## First Public Release Checklist

1. Choose the project license and add `LICENSE` plus `license` or `license-file` in `Cargo.toml`.
2. Ensure `main` is green and the release commit is the intended commit.
3. Create and push `v0.1.0`.
4. Confirm the GitHub Release has all archives, checksums, and `stellar-forge.rb`.
5. Run the workflow manually with `publish_external` only after the protected environments and
   channel credentials are configured.
6. Start with crates.io, then Homebrew tap, then `.deb` artifacts. Keep APT and Snap blocked until
   signing, hosting, snap ownership, and confinement are explicit.
