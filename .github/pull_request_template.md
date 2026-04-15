## Summary

- what problem this change solves
- what changed in the CLI, docs, or generated output
- why this is the right scope for the change

## User impact

- what users will notice
- whether JSON reports, generated files, or release artifacts changed
- any migration or regeneration steps users should take

## Validation

List the exact commands you ran:

```bash
# example
cargo fmt --all
cargo test --locked --test cli
```

Checklist:

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo audit`

## Files to review

- implementation:
- tests:
- docs:
- generated output:

## Checklist

- [ ] I updated docs, examples, or generated fixtures when behavior changed
- [ ] I added or updated tests for user-visible behavior
- [ ] I called out any follow-up work, trade-offs, or known limitations
- [ ] I noted any required external tooling such as `stellar`, Docker, Node.js, or `sqlite3`
- [ ] I mentioned if this diff intentionally includes generated files or release artifacts

## Follow-up notes

- risks:
- future cleanup:
- reviewer context:
