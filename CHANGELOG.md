# Changelog

All notable changes to `stellar-forge` are documented in this file.

## 0.1.0 - 2026-06-02

Initial public release of `stellar-forge`, a Rust CLI for manifest-driven Stellar workspaces.

### Added

- Project bootstrap templates for minimal contracts, fullstack apps, issuer wallets, merchant checkout, rewards loyalty, API-only, and multi-contract workspaces.
- Manifest-driven validation, project synchronization, dry-run planning, and generated helper files around `stellarforge.toml`.
- Lockfile and release artifact support through `stellarforge.lock.json`, `.env.generated`, `dist/deploy.<env>.json`, and release history helpers.
- Contract workflows for build, deploy, invoke, bindings, TTL, fetch, formatting, and linting through the official `stellar` CLI.
- Wallet, SEP-7, relayer, batch-payment, smart-wallet, token, SAC wrapper, and airdrop command families.
- Generated API, OpenAPI, relayer, frontend, and event-ingestion scaffolds with smoke-check support.
- Local development helpers for diagnostics, reseeding, smoke checks, release status, diff, drift, rollback, env export, and alias synchronization.
- GitHub release pipeline that packages Linux x86_64 and macOS x86_64/aarch64 binary archives with SHA-256 checksum files.
