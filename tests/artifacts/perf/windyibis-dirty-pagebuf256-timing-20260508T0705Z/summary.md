# Dirty Integration Focused Profiles

Date: 2026-05-08
Agent: WindyIbis
Base commit: `8e1b404fb3eaa88f9920570d12683fccff41d24b`
Tree state: dirty, with `GLOBAL_PAGE_BUF_RECYCLE_CAPACITY` locally restored to
`256` and the WAL/e2e commit-phase timing toggle follow-up present.
Binary modified: `2026-05-08 06:59:19 UTC`

## Commands

- `env TMPDIR=/data/tmp/frankensqlite-windyibis-tmp CARGO_TARGET_DIR=.rch-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
- `env FSQLITE_BENCH_PROFILE_INSERT=1 .rch-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-20260508T0705Z/insert-profile.json`
- `env FSQLITE_BENCH_PROFILE_DML=1 .rch-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-20260508T0705Z/update-profile.json`
- `.rch-target/release-perf/comprehensive-bench --quick --filter concurrent --no-html --json-out tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-20260508T0705Z/concurrent-profile.json`

## INSERT

Artifact: `insert-profile.json`

- Weighted score: `0.7102431265064252`
- Average ratio: `0.7855038130421153`
- Geomean ratio: `0.7641072292916575`
- P90 ratio: `1.0837908998293222`
- P99 ratio: `1.1024115025992243`
- Faster/comparable/slower: `18 / 2 / 5`

The page-buffer cap restore removes the 2048-cap regression: the large
10-column record-size row is back near parity:

- C SQLite median: `9.572926 ms`
- FrankenSQLite median: `9.972735 ms`
- Ratio: `1.0417645555810209`

Remaining INSERT rows above parity are small fixed-overhead 100-row cases plus
near-parity large-row tails.

## UPDATE/DELETE

Artifact: `update-profile.json`

- Weighted/geomean score: `1.0467937414780393`
- Average ratio: `1.0617534252055076`
- P90/P99 ratio: `1.3307941899667926`
- Faster/comparable/slower: `3 / 1 / 2`

The slower rows are the 100-row cases:

- `100 rows / update 10 rows`: C `86.2 us`, F `113.0 us`, ratio `1.31x`
- `100 rows / delete 5 rows`: C `78.9 us`, F `105.0 us`, ratio `1.33x`

FSQLite profile details show these are setup-dominated, not mutation-bound:

- Update setup `51.2 us`, mutate `12.2 us`
- Delete setup `52.3 us`, mutate `8.6 us`

The negative ledger already rejects several standalone setup/open ideas
against this family, including lazy waiter shards, lazy conflict ring
allocation, stack page-1 bootstrap, exact PRAGMA fast paths, and direct-DML
schema microbatch carry.

## Concurrent Writers

Artifact: `concurrent-profile.json`

- Weighted/geomean score: `0.8251996968872611`
- Average ratio: `0.8798875349570152`
- P90/P99 ratio: `1.1135394153191414`
- Faster/comparable/slower: `1 / 1 / 1`

Rows:

- `2 writers x 1000 rows`: C `12.40 ms`, F `13.81 ms`, ratio `1.1135x`
- `4 writers x 1000 rows`: C `19.07 ms`, F `19.86 ms`, ratio `1.0417x`
- `8 writers x 1000 rows`: C `90.89 ms`, F `44.03 ms`, ratio `0.4845x`

## Decision

Evidence-only profile bundle. This supports landing the local page-buffer cap
restore to `256`, but it is not a standalone new optimization keep.
