# Current Post-DML Baseline

Date: 2026-05-08

Head: `a2a7ae80 docs(perf): record direct write probe gates`

Binary:
`/data/tmp/frankensqlite-current-post-dml-target/release-perf/comprehensive-bench`

Build command:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-post-dml-target CARGO_BUILD_JOBS=10 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Full quick command:

```text
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-current-post-dml-target/release-perf/comprehensive-bench \
  --quick --no-html \
  --json-out tests/artifacts/perf/current-post-dml-tanbear-20260508T0110Z/full-quick-current.json
```

Focused DML command:

```text
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-current-post-dml-target/release-perf/comprehensive-bench \
  --quick --filter update --no-html \
  --json-out tests/artifacts/perf/current-post-dml-tanbear-20260508T0110Z/update-profile-current.json
```

## Full Quick Summary

- Total scenarios: `93`
- FrankenSQLite faster: `81`
- Comparable: `5`
- C SQLite faster: `7`
- Average ratio: `0.46176051564782017`
- Geomean ratio: `0.27117173510593306`
- P90 ratio: `0.9989961423183377`
- P99 ratio: `1.3717396123248196`
- Primary weighted score: `0.34795771156443783`

Remaining C-SQLite-faster rows above `1.05x`:

| Ratio | Section | Scenario | Category | FSQLite ms | C SQLite ms |
| ---: | --- | --- | --- | ---: | ---: |
| `1.371740` | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | write_single | `0.111388` | `0.081202` |
| `1.249011` | INSERTThroughput - Single Transaction - medium_6col | 1000 rows | write_bulk | `0.672891` | `0.538739` |
| `1.178643` | INSERTThroughput - Single Transaction - small_3col | 100 rows | write_bulk | `0.088245` | `0.074870` |
| `1.170732` | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | write_bulk | `0.087113` | `0.074409` |
| `1.147442` | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | write_bulk | `0.085380` | `0.074409` |
| `1.111222` | INSERTThroughput - Single Transaction - large_10col | 100 rows | write_bulk | `0.162064` | `0.145843` |
| `1.079294` | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | concurrent_writers | `14.098936` | `13.063106` |

## Focused UPDATE/DELETE Profile

Focused section summary:

- Total scenarios: `6`
- FrankenSQLite faster: `0`
- Comparable: `2`
- C SQLite faster: `4`
- Average ratio: `1.1579451025994136`
- Geomean ratio: `1.1460254210421432`
- P90/P99 ratio: `1.4599792299703835`

Rows:

| Ratio | Scenario | FSQLite ms | C SQLite ms |
| ---: | --- | ---: | ---: |
| `1.459979` | 100 rows / delete 5 rows | `0.113874` | `0.077997` |
| `1.325330` | 100 rows / update 10 rows | `0.119224` | `0.089958` |
| `1.074879` | 10000 rows / delete 500 rows | `3.475211` | `3.233118` |
| `1.050288` | 10000 rows / update 1000 rows | `3.867106` | `3.681948` |
| `1.039226` | 1000 rows / delete 50 rows | `0.372148` | `0.358101` |
| `0.997968` | 1000 rows / update 100 rows | `0.403266` | `0.404087` |

Profile phase timings show the small DML rows are dominated by setup and
statement ceremony, not the mutation kernel alone:

| Row | setup_us | begin_us | prepare_us | mutate_us | commit_us |
| --- | ---: | ---: | ---: | ---: | ---: |
| `fs_update_100` | `56.1` | `7.0` | `13.0` | `12.0` | `6.0` |
| `fs_delete_100` | `55.8` | `5.0` | `11.9` | `8.4` | `5.1` |
| `fs_update_1000` | `263.2` | `6.7` | `13.6` | `96.4` | `9.2` |
| `fs_delete_1000` | `245.4` | `5.2` | `12.0` | `66.3` | `8.2` |
| `fs_update_10000` | `2136.0` | `7.6` | `16.4` | `1051.3` | `92.0` |
| `fs_delete_10000` | `2478.2` | `15.1` | `26.3` | `757.1` | `254.6` |

## Interpretation

The direct DML fast path has already removed the old table-program compile
penalty. The worst remaining row (`100 rows / delete 5 rows`) still looks bad
in the full matrix, but its focused profile spends only `8.4 us` mutating rows
versus `55.8 us` in setup and `11.9 us` in prepare. Standalone direct DELETE
kernel ideas are therefore low expected value unless they also remove setup,
prepopulation, or broader row-build costs.

The highest-EV remaining write target is the direct prepared INSERT row
construction surface. The remaining INSERT gaps cluster around small-N and
medium record-size prepared inserts, and the profile shows per-row work in
`row_build_ns`, `schema_validation_ns`, `change_tracking_ns`, and
`memdb_apply_ns`. Previous ledger entries reject narrow param-one expression
specialization, lazy text caching, page-run admission changes, and standalone
direct-DML cursor changes, so a retry should be a broader row-template or
execution-fusion design with same-window INSERT and full-quick gates.
