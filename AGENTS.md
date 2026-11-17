# sf1-port

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (via the `gh` CLI). See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.

## Tests

Run the full suite with `cargo suite`: the whole workspace, optimized, into `target/suite`, without stopping at the first failing file. Plain `cargo test` stays the quick path for single test files. Stack-depth failures can pass optimized and still fail in a debug build, so check those with plain `cargo test`.
