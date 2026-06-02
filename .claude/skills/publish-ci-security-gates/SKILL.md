---
name: publish-ci-security-gates
description: Use when stellar-forge release publishing touches GitHub Actions permissions, secrets, protected environments, token scope, signing keys, provenance, OIDC, registry credentials, or accidental publish prevention.
---

# Publish CI Security Gates

## Core Principle

Release automation should make the safe path easy and the dangerous path impossible. Real registry credentials belong behind trusted release events, preflight checks, and protected environments.

## Security Review Checklist

1. Confirm publish jobs never run on `pull_request`.
2. Confirm the tag looks like `vMAJOR.MINOR.PATCH` and matches `Cargo.toml`.
3. Keep `permissions` minimal per job; add `contents: write` only where a job updates releases or tap repos.
4. Keep registry tokens in secrets or environments, not workflow inputs.
5. Prefer one secret per registry: crates.io, tap repo, Snap Store, APT signing/upload.
6. Print whether secrets are present, but never print their values.
7. Add manual approval environments for irreversible external publishes.
8. Record partial publish state when one channel succeeds and another blocks.

## Secret Matrix

| Secret or environment | Used by | Notes |
| --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | crates.io | registry-scoped token |
| `HOMEBREW_TAP_TOKEN` | Homebrew tap | repo-scoped token or GitHub App token |
| `SNAPCRAFT_STORE_CREDENTIALS` | Snap Store | generated store login credentials |
| `APT_SIGNING_KEY` | APT repo | armored private key or environment-managed signing key |
| `APT_SIGNING_KEY_ID` | APT repo | signing identity for repository metadata |
| `APT_REPO_UPLOAD_TOKEN` | APT repo | host-specific upload credential |

## Common Mistakes

- Using `GITHUB_TOKEN` where a cross-repository tap update requires a scoped token.
- Adding publish secrets to a workflow that also runs on PRs.
- Letting a missing optional secret fail after build minutes instead of preflighting.
- Treating package signing as optional for an APT repository.

## Verification

Use static checks first:

```bash
rtk rg -n "pull_request|CARGO_REGISTRY_TOKEN|SNAPCRAFT|APT_|HOMEBREW|permissions:" .github/workflows
rtk git diff --check
```
