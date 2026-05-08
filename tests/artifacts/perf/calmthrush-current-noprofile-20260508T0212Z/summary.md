# Current No-Profile Full Quick Baseline

Date: 2026-05-08

Head:
`63e30bc2d7876cc8c1e53da06d774701316ecde9`
(`docs(perf): reject memdb abandon skip probe`)

Purpose: refresh the full quick matrix without
`FSQLITE_BENCH_PROFILE_INSERT` or `FSQLITE_BENCH_PROFILE_DML`, so target
selection is not skewed by profiling-only hot-path counters.

## Commands

Build:

```text
env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-noprofile-target \
  CARGO_BUILD_JOBS=8 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Run:

```text
/data/tmp/frankensqlite-current-noprofile-target/release-perf/comprehensive-bench \
  --quick --no-html \
  --json-out tests/artifacts/perf/calmthrush-current-noprofile-20260508T0212Z/full-quick-noprofile.json
```

## Summary

- Total scenarios: `93`
- FrankenSQLite faster: `79`
- Comparable: `3`
- C SQLite faster: `11`
- Average ratio: `0.4749143476202728`
- Geomean ratio: `0.27937915287482756`
- P90 ratio: `1.0547901508362068`
- P99 ratio: `1.3855662763006065`
- Primary weighted score: `0.3626839488398201`

Rows still above `1.05x`:

| Ratio | Section | Scenario | Category | FSQLite ms | C SQLite ms |
| ---: | --- | --- | --- | ---: | ---: |
| `1.385566` | UPDATE/DELETEThroughput | 100 rows / update 10 rows | write_single | `0.112871` | `0.081462` |
| `1.374662` | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | write_single | `0.108953` | `0.079258` |
| `1.237048` | INSERTThroughput - Single Transaction - medium_6col | 1000 rows | write_bulk | `0.696933` | `0.563384` |
| `1.167151` | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | write_bulk | `0.085209` | `0.073006` |
| `1.140006` | INSERTThroughput - Single Transaction - small_3col | 100 rows | write_bulk | `0.083787` | `0.073497` |
| `1.133836` | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | concurrent_writers | `13.497917` | `11.904647` |
| `1.119657` | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | write_bulk | `0.082203` | `0.073418` |
| `1.112629` | INSERTThroughput - Single Transaction - large_10col | 100 rows | write_bulk | `0.160430` | `0.144190` |
| `1.055536` | INSERTThroughput - Single Transaction - large_10col | 10000 rows | write_bulk | `9.958005` | `9.434075` |
| `1.054790` | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | write_bulk | `0.069440` | `0.065833` |
| `1.054225` | INSERTThroughput - Single Transaction - medium_6col | 100 rows | write_bulk | `0.107902` | `0.102352` |

Near-threshold rows:

| Ratio | Section | Scenario | Category | FSQLite ms | C SQLite ms |
| ---: | --- | --- | --- | ---: | ---: |
| `1.023280` | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B) | write_bulk | `9.497144` | `9.281080` |
| `1.000509` | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | write_single | `0.371464` | `0.371275` |
| `0.959522` | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | write_single | `0.390470` | `0.406942` |

## Interpretation

The no-profile matrix makes the small DML setup rows the top true remaining
gap: 100-row UPDATE and DELETE are both about `1.38x` slower than C SQLite.
The next cluster remains prepared direct INSERT, with medium 1000-row and
small 100-row cases above `1.1x`.

This differs from the profile-enabled full quick artifact, where the largest
reported gap was small DELETE and the UPDATE row did not appear in the remaining
full-matrix list. Future source candidates should use this no-profile artifact
for target ordering, then add focused profile instrumentation only to explain a
chosen target.

At capture time, `crates/fsqlite-core/src/connection.rs`,
`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`, and
`docs/progress/perf-negative-results.md` were reserved by CrimsonGorge, so this
pass intentionally published target-selection evidence only.
