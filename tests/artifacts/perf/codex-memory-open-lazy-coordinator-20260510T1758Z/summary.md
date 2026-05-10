# Memory Open Lazy Coordinator Probe - 2026-05-10

## Candidate

Skip creating the `WriteCoordinator` runtime region for private `:memory:`
shared MVCC state, leaving file-backed shared state unchanged. The intended
target was fresh-memory setup/open overhead visible in the remaining 100-row
INSERT and setup-heavy rows.

The source candidate touched `crates/fsqlite-core/src/connection.rs` and was
reverted after measurement. The retained source change from this pass is
test-only: the write-coordinator tests now assert the existing runtime-scoped
shared-state contract directly.

## Correctness Gate

- `cargo fmt -p fsqlite-core` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-lazy-coordinator-local-target cargo check -p fsqlite-core --lib`
  passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-lazy-coordinator-local-target cargo test -p fsqlite-core write_coordinator -- --nocapture`
  passed locally: 11 matching tests passed.
- An earlier RCH run of the same focused test lane also printed the 11-test
  pass result, but the local `rch` wrapper hung during artifact retrieval after
  the remote command reported `exit=0`; the wrapper process was interrupted
  locally and is not used as the final proof.

## Focused Benchmark Gate

Command:

```text
/data/tmp/frankensqlite-codex-lazy-coordinator-local-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/codex-memory-open-lazy-coordinator-20260510T1758Z/insert.json --no-html
```

Comparator: latest frontier INSERT profile
`tests/artifacts/perf/codex-fresh-frontier-insert-profile-20260510T093306Z/insert.json`.

Candidate summary:

- total scenarios: `25`
- FrankenSQLite faster/comparable/C faster: `17 / 2 / 6`
- weighted score: `0.8259408671941716`
- geomean ratio: `0.8403074799499317`
- p90 / p99 ratio: `1.1668266857812597 / 1.2073917903584113`

Comparator summary:

- total scenarios: `25`
- FrankenSQLite faster/comparable/C faster: `17 / 1 / 7`
- weighted score: `0.8363653414261966`
- geomean ratio: `0.8354798325860695`
- p90 / p99 ratio: `1.1328306804430792 / 1.1746661184864922`

## Decision

Reject as a standalone source change.

The weighted score moved slightly in the right direction, but the movement came
with worse p90/p99 and broad absolute FrankenSQLite median regressions. The
candidate worsened absolute FrankenSQLite time in most compared rows, including
`small_3col` 10K single transaction (`2.568016 ms -> 3.674636 ms`), `medium_6col`
1K (`0.429053 ms -> 0.475449 ms`), `large_10col` 100 (`0.177142 ms -> 0.238286 ms`),
and record-size `large_10col` 10K (`10.197313 ms -> 11.481385 ms`). That is not
a valid keep for a setup/open optimization.

Do not retry "skip private memory write-coordinator region" as a standalone
optimization. Reconsider only inside a broader open-state redesign with a
same-window baseline/candidate run that improves absolute FrankenSQLite medians
on the setup-heavy INSERT rows and does not worsen full quick p90/p99.
