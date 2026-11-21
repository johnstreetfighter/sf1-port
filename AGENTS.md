# sf1-port

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (via the `gh` CLI). See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.

## Tests

Run the full suite with cargo-nextest, in two passes, one after the other: `cargo suite-parallel` (everything that can share the machine, on 8 threads, longest tests first), then `cargo suite-serial` (the network, relay and sf-host library tests, one at a time). Both build the whole workspace optimized into `target/suite`, and run to the end without stopping at the first failure; `.config/nextest.toml` holds the setup. Together they take about 45 minutes and run every test `cargo suite` does. `cargo suite` (cargo test, one binary at a time, about 90 minutes) is still there when you need it.

The full suite passes with no known failures. Don't run it beside MAME recordings or another suite: this machine has 15 GB of RAM and has run out doing that. Exit codes 0xc000012d, 0xc0000409 or 0xc0000142 mean it ran out of memory, not that a test regressed.

Plain `cargo test` stays the quick path for single test files. Stack-depth failures can pass optimized and still fail in a debug build, so check those with plain `cargo test`.
