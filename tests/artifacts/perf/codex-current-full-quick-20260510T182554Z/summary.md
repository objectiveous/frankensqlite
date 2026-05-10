# Current Full-Quick Frontier Refresh

Date: 2026-05-10 UTC

Source: `abc82a1b98ec2bebfb7538927d727c0111d38103`

Command:

```bash
/data/tmp/frankensqlite-codex-after-63cf-current-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/codex-current-full-quick-20260510T182554Z/full-quick.json \
  --no-html
```

The binary was built earlier from the same production source tree with:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-after-63cf-current-target \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Raw command output is retained in `stdout.log` and `stderr.log`.

## Summary

| Metric | Value |
| --- | ---: |
| Total scenarios | 93 |
| FrankenSQLite faster | 78 |
| Comparable | 2 |
| C SQLite faster | 13 |
| Average ratio | 0.53144 |
| Geomean ratio | 0.28559 |
| Median ratio | 0.29035 |
| p90 ratio | 1.14266 |
| p99 ratio | 3.66681 |
| Weighted primary score | 0.38954 |

Lower ratios are better for FrankenSQLite.

## Category Summary

| Category | Rows | Geomean | Median | p90 |
| --- | ---: | ---: | ---: | ---: |
| `concurrent_writers` | 3 | 0.82967 | 1.01686 | 1.16009 |
| `mixed` | 1 | 0.18270 | 0.18270 | 0.18270 |
| `read_aggregate` | 25 | 0.07901 | 0.13416 | 0.50478 |
| `read_single` | 33 | 0.21695 | 0.21209 | 0.30131 |
| `write_bulk` | 22 | 0.88042 | 0.88304 | 1.17168 |
| `write_single` | 9 | 1.30490 | 0.90890 | 3.66681 |

## C-Faster Rows

| Ratio | Section | Scenario | C median ms | F median ms | C CV % | F CV % |
| ---: | --- | --- | ---: | ---: | ---: | ---: |
| 3.66681 | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.002254 | 0.008265 | 2.42 | 2.69 |
| 2.17638 | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 0.015739 | 0.034254 | 1.87 | 7.68 |
| 2.01651 | UPDATE/DELETEThroughput | 10000 rows / delete 500 rows | 0.158968 | 0.320560 | 2.98 | 28.60 |
| 1.68098 | UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.004238 | 0.007124 | 0.76 | 3.15 |
| 1.47774 | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.075100 | 0.110978 | 28.00 | 12.64 |
| 1.17195 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.073417 | 0.086041 | 4.60 | 5.91 |
| 1.17168 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.074529 | 0.087324 | 7.87 | 17.62 |
| 1.16009 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 12.007328 | 13.929567 | 21.63 | 18.15 |
| 1.14364 | INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.146965 | 0.168075 | 6.86 | 3.02 |
| 1.14266 | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B: includes long text fields) | 9.787312 | 11.183606 | 0.81 | 5.10 |
| 1.13259 | INSERTThroughput - Single Transaction - large_10col | 10000 rows | 9.542624 | 10.807882 | 0.51 | 1.43 |
| 1.09006 | INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.101671 | 0.110827 | 7.55 | 8.15 |
| 1.06186 | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.066575 | 0.070693 | 4.91 | 6.03 |
| 1.01686 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 4 writers x 1000 rows | 19.741246 | 20.074139 | 13.39 | 12.05 |

## Decision

No source patch was attempted from this refresh. The current top stable gap is
still the DML mutation frontier. Prior same-session artifacts already reject the
nearby one-lever DELETE shapes: standalone leaf-run admission tweaks, retained
cursor hints, scanned dirty-leaf backlogs, disabling the leaf run, next-cell
hints, and repeated-seek rowid buffering.

The remaining credible DML target is a broader transaction-level mutation
representation that can prove exact affected-row counts, read-your-writes,
savepoint/rollback behavior, failed-flush preservation, quotient-filter
invalidation, and batched pager/MVCC publication. The small INSERT rows should
be repeated before using them as source targets because at least one red
`small_3col` row has very high C SQLite variance.
