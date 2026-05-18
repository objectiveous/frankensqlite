# INSERT fixed-cost profile refresh

Date: 2026-05-18
Commit: `c41a4175`
Raw results: `insert.json`
Run log: `run.log`

Command:

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_INSERT=1 CARGO_TARGET_DIR=/data/tmp/frankensqlite-insert-fixedcost-target-c41a4175-20260518 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-insert-fixedcost-profile-c41a4175-20260518T0100Z/insert.json --no-html
```

`rch` fell back to local execution because no worker was admissible (`critical_pressure=6`).

## Summary

The focused INSERT slice now reports 25 scenarios: 12 FrankenSQLite faster, 7 comparable, and 6 C SQLite faster. Overall geomean ratio is 0.9073x F/C, median is 0.9656x, and the per-category weighted score is 1.0531x across the observed write-only categories.

This refresh changes the target selection from the older "small 100-row fixed cost" framing. The tiny/small single-transaction rows are mostly comparable or faster now; the remaining durable source-level work is still row/body/page construction for medium and large rows.

## Red Rows

| Ratio F/C | Section | Scenario | Category | C median ms | F median ms | C CV % | F CV % |
| ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| 1.5449 | Transaction Strategy Comparison (small_3col) | 100 rows / autocommit | write_single | 0.1736 | 0.2682 | 48.0 | 24.0 |
| 1.3289 | Transaction Strategy Comparison (small_3col) | 100 rows / single txn | write_bulk | 0.0906 | 0.1204 | 28.5 | 40.2 |
| 1.3071 | Single Transaction medium_6col | 100 rows | write_bulk | 0.1883 | 0.2461 | 46.5 | 33.4 |
| 1.2987 | Single Transaction medium_6col | 1000 rows | write_bulk | 0.5740 | 0.7455 | 11.5 | 22.4 |
| 1.1533 | Single Transaction large_10col | 10000 rows | write_bulk | 9.9991 | 11.5322 | 6.5 | 7.2 |
| 1.0817 | Single Transaction large_10col | 100 rows | write_bulk | 0.1886 | 0.2041 | 33.6 | 44.0 |
| 1.0474 | Record Size Comparison | large_10col 10K rows | write_bulk | 10.1362 | 10.6164 | 4.1 | 3.3 |

## Hotspots From Profile Lines

Stable large-row rows continue to point at fused row/body/page construction:

| Scenario | Rows | Insert us | Commit us | Row build ns | Preserialize ns | Preserialize cell ns | Direct flush ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `fs_insert_single_txn_medium_6col_1000` | 1000 | 1759.1 | 101.7 | 1285305 | 1222831 | 1073662 | 57377 |
| `fs_insert_single_txn_medium_6col_10000` | 10000 | 16366.7 | 1127.1 | 11780995 | 11206241 | 9809141 | 762608 |
| `fs_insert_record_size_large_10col_10000` | 10000 | 24586.1 | 5105.8 | 19801755 | 19196976 | 17525928 | 2620328 |

## Decision

No source patch is kept from this run. The next candidate should not be another isolated serializer/template/scratch tweak; the current profile and the negative-results ledger both point to a broader fused row/body/page construction path that reduces per-cell planning and avoids building an owned record vector before bulk leaf construction.
