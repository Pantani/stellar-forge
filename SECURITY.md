# Security Policy

## Supported versions

`stellar-forge` is still in active early development.

Security fixes are prioritized for:

- the latest code on the default branch
- the current `0.1.x` release line

| Version | Supported |
| --- | --- |
| `0.1.x` | Yes |
| older versions | Best effort only |

If you are unsure whether a report applies to a supported version, send it anyway.
The maintainers can help decide how it should be triaged.

## What counts as a security issue

Please use private reporting for issues such as:

- arbitrary command execution paths that cross expected trust boundaries
- unintended file write or file disclosure behavior
- secrets or credential leakage
- unsafe deploy or rollback behavior that can be triggered unexpectedly
- authorization or policy bypass in generated relayer or API scaffolds
- supply-chain or dependency issues with practical impact on users

Normal bugs, usability problems, and missing features should go through the usual public issue
templates instead.

## How to report a vulnerability

Please do not open a public GitHub issue for undisclosed vulnerabilities.

Instead, use one of these channels:

1. GitHub private vulnerability reporting, if it is enabled for this repository
2. a private contact method exposed by the repository owner or maintainers

If no private reporting channel is available, open a minimal public issue asking for a secure
contact path without including the vulnerability details.

## What to include

Please include:

- a clear description of the issue
- affected versions, tags, or commit hashes
- attack prerequisites and realistic impact
- proof of concept or reproduction steps, when safe to share
- whether the issue depends on local tooling such as `stellar`, Docker, Node.js, or `sqlite3`
- any suggested mitigation or fix ideas, if you already have them

Especially useful:

- exact commands used to reproduce
- the smallest possible manifest or project fixture
- whether the problem occurs in `--dry-run` mode too
- whether generated files, deploy artifacts, or lockfile state are involved

## Safe reporting guidance

When possible:

- avoid posting secrets, tokens, private keys, or live credentials
- use redacted logs if full logs contain sensitive data
- prefer testnet or local-network proof of concept material over production data
- state clearly if exploitation requires unusual local setup or elevated access

## What to expect from maintainers

Maintainers will try to:

1. acknowledge the report quickly
2. confirm whether the issue is in scope
3. assess severity and affected versions
4. coordinate a fix before public disclosure
5. publish remediation guidance when a fix is available

Response times can vary, but well-scoped reports with reproduction details are much easier to act
on quickly.

## Disclosure expectations

Please allow reasonable time for investigation, patching, validation, and coordinated disclosure.

If the issue affects generated scaffolds, releases may need time not only for a code fix but also
for documentation updates and guidance on how existing users should regenerate or patch their
workspaces.

## After a fix lands

Security fixes may involve one or more of:

- a code change in the CLI
- updated generated templates
- documentation updates
- release notes or migration guidance

If your report affects how users should rotate secrets, regenerate output, or inspect deploy state,
please mention that in the report. That context helps produce a better remediation plan.
