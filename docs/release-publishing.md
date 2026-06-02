# Release Publishing

`stellar-forge` publishes GitHub Release binaries first, then fans out to package-manager channels
from the same tag.

## Pipeline Shape

`.github/workflows/release.yml` runs on:

- pushed tags matching `vMAJOR.MINOR.PATCH` with optional prerelease suffixes
- manual `workflow_dispatch` with an existing tag

The release core is the only hard gate. It builds Linux x86_64/aarch64 and macOS x86_64/aarch64
archives, attaches checksums and `LICENSE`, and publishes the GitHub Release. After that, package
channels run best-effort with `continue-on-error: true`.

```text
package matrix -> release -> publish-preflight -> crates.io
                                           |-> homebrew-formula -> publish-homebrew
                                           |-> publish-apt    (matrix: amd64, arm64)
                                           |-> publish-snap   (matrix: amd64, arm64)
                                           |-> release-summary
```

Missing channel secrets degrade to build-only or artifact-only instead of failing the release. The
`release-summary` job reports every channel in the Job Summary tab and fails only when the GitHub
Release job itself failed.

## Install Commands

After the relevant channel has published:

```bash
# GitHub Release archive
curl -L https://github.com/Pantani/stellar-forge/releases/download/v0.1.0/stellar-forge-0.1.0-x86_64-unknown-linux-gnu.tar.gz | tar -xz
install -m 0755 stellar-forge-0.1.0-x86_64-unknown-linux-gnu/stellar-forge ~/.local/bin/stellar-forge

# crates.io
cargo install stellar-forge

# Homebrew
brew install Pantani/tap/stellar-forge

# Snap Store
sudo snap install stellar-forge --classic

# APT via Cloudsmith
curl -1sLf 'https://dl.cloudsmith.io/public/pantani/stellar-forge/setup.deb.sh' | sudo -E bash
sudo apt-get install stellar-forge
```

## Preflight Checks

The publish preflight validates:

- release tag format
- `Cargo.toml` version matches the tag without the `v`
- `CHANGELOG.md` has a section for the version
- expected release archives and `.sha256` files exist
- package license metadata exists
- `Pantani/homebrew-tap` or `HOMEBREW_TAP_REPO` exists
- `snap/snapcraft.yaml` exists

## Channels

| Channel | CI behavior | Secret or setup |
| --- | --- | --- |
| GitHub Release binaries | hard-fail release core | tag points at intended commit |
| crates.io | dry-run always, publish only if token exists, skip if version already exists | `CARGO_REGISTRY_TOKEN` |
| Homebrew formula artifact | automatic, attached to GitHub Release | none |
| Homebrew tap | pushes `Formula/stellar-forge.rb` directly when token exists | `HOMEBREW_TAP_TOKEN`, `HOMEBREW_TAP_REPO` defaults to `Pantani/homebrew-tap` |
| APT / Cloudsmith | builds amd64 and arm64 `.deb`, attaches them to the GitHub Release, pushes to Cloudsmith if token exists | `CLOUDSMITH_API_KEY`, optional `CLOUDSMITH_OWNER`, `CLOUDSMITH_REPO` |
| Snap Store | builds amd64 and arm64 snaps, attaches them to the GitHub Release, publishes if credentials exist | `SNAPCRAFT_STORE_CREDENTIALS`, optional `SNAP_CHANNEL` |

## GitHub Environments

The workflow references these environments:

- `github-release`
- `crates-io`
- `homebrew-tap`
- `apt-repo`
- `snap-store-stable`

Add required reviewers and prevent self-review before storing production publish credentials. Keep
PR workflows read-only and never expose publish credentials to `pull_request` events.

## Secrets and Variables

| Name | Type | Used by |
| --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | environment secret | crates.io publish |
| `HOMEBREW_TAP_TOKEN` | environment secret | pushing formula to the tap |
| `SNAPCRAFT_STORE_CREDENTIALS` | environment secret | Snap Store upload/release |
| `CLOUDSMITH_API_KEY` | environment secret | Cloudsmith Debian repository upload |
| `HOMEBREW_TAP_REPO` | repo or environment variable | override `Pantani/homebrew-tap` |
| `CLOUDSMITH_OWNER` | repo or environment variable | override `pantani` |
| `CLOUDSMITH_REPO` | repo or environment variable | override `stellar-forge` |
| `SNAP_CHANNEL` | repo or environment variable | override `stable` |

Generate Snap credentials with:

```bash
snapcraft login
snapcraft export-login --snaps=stellar-forge --channels=stable snap-creds.txt
```

Cloudsmith expects a public Debian repository at `pantani/stellar-forge` unless the owner or repo
variables are overridden. The workflow uploads with `--republish` so rerunning the same tag can
replace the same version/architecture tuple.

## Rerunning a Channel

Rerun the workflow with the existing tag:

```bash
gh workflow run release.yml -f tag=v0.1.0
```

This is safe to use for backfilling `v0.1.0` after the CI changes land:

- crates.io checks whether the version already exists before publishing
- Homebrew commits only when the formula content changed
- Cloudsmith uses `--republish`
- Snap Store deduplicates identical revisions or reports the channel issue without blocking the
  GitHub Release
