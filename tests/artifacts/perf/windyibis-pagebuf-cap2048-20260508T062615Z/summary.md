# PageBuf Capacity 2048 Focused INSERT Retest

Date: 2026-05-08
Agent: WindyIbis
Candidate: `GLOBAL_PAGE_BUF_RECYCLE_CAPACITY` raised from `256` to `2048`
Commit under test: `41a950b6326d56ab14bf49148d56ea9e5eebfa6e`

## Commands

- `cargo fmt -p fsqlite-pager --check`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-pagebuf-cap2048-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-pager page_buf -- --nocapture`
- `env TMPDIR=/data/tmp/frankensqlite-windyibis-tmp CARGO_TARGET_DIR=.rch-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
- `env FSQLITE_BENCH_PROFILE_INSERT=1 .rch-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/windyibis-pagebuf-cap2048-20260508T062615Z/insert-profile.json`

## Prior Keeper

Artifact:
`tests/artifacts/perf/windyibis-schema-index-plan-20260508T055049Z/pagebuf-insert-profile.json`

- Weighted score: `0.7767315568388111`
- Average ratio: `0.7714626475032516`
- Geomean ratio: `0.7459511333726486`
- P90 ratio: `1.0760814249363868`
- P99 ratio: `1.0984144114228789`
- Faster/comparable/slower: `19 / 2 / 4`

## Candidate

Artifact:
`tests/artifacts/perf/windyibis-pagebuf-cap2048-20260508T062615Z/insert-profile.json`

- Weighted score: `0.8170165218916904`
- Average ratio: `0.9271209597934807`
- Geomean ratio: `0.8792631315813308`
- P90 ratio: `1.2516524592352403`
- P99 ratio: `2.042096610168269`
- Faster/comparable/slower: `16 / 1 / 8`

Worst row:

- `INSERTThroughput - Record Size Comparison (10K rows, single txn) / large_10col`
- C SQLite median: `9.706411 ms`
- FrankenSQLite median: `19.821429 ms`
- Ratio: `2.042096610168269`
- Profile still showed `page_pool_misses=2006`.

## Decision

Rejected. The larger global recycle cap did not reduce the large-row page-pool
miss source and made the focused INSERT section materially worse. Do not treat
the 2048-cap change as a keeper without a fresh revert or retarget proof.
