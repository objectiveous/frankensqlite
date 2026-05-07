# Current HEAD INSERT profile after lazy VersionStore

Profile captured from `main` at `6d6c84279b1dec4baf4564122e113396495ed91b`
after the retained autocommit, lock-table shard, and lazy `VersionStore`
changes had landed.

Command:

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-purpleotter-lockshards64-perf-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/insert-current-after-versionstore-purpleotter-20260507T0712Z/report-insert.json \
  --no-html
```

## Section summary

| Metric | Value |
| --- | ---: |
| Total INSERT scenarios | 25 |
| FrankenSQLite faster | 13 |
| Comparable | 1 |
| C SQLite faster | 11 |
| Average ratio | 0.969046 |
| Geomean ratio | 0.936488 |
| Median ratio | 0.923666 |
| P90 ratio | 1.305005 |
| P99 ratio | 1.334508 |
| INSERT weighted score | 1.165727 |
| Write-bulk geomean | 0.898233 |
| Write-single geomean | 1.271548 |

## Remaining slow INSERT rows

The stable remaining gap is transaction-strategy `small_3col`, especially:

| Row | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / autocommit | 123.0 us | 148.7 us | 1.21x slower |
| 1000 rows / autocommit | 850.6 us | 1.11 ms | 1.31x slower |
| 10000 rows / autocommit | 8.25 ms | 10.75 ms | 1.30x slower |
| 10000 rows / batched (1000/txn) | 3.52 ms | 4.69 ms | 1.33x slower |

## Profiler signal

`FSQLITE_BENCH_PROFILE_INSERT=1` currently covers single-transaction row-count
and record-size insert paths, not the transaction-strategy autocommit loop.
Useful samples from `report-insert.stderr`:

| Profile row | Insert | Commit | Row build | B-tree | MemDB | Schema validation | Change tracking | Page misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `single_txn_tiny_1col_10000` | 4.643 ms | 0.420 ms | 0.325 ms | 0.327 ms | 0.240 ms | 0.314 ms | 0.239 ms | 19 |
| `single_txn_small_3col_10000` | 6.146 ms | 0.385 ms | 1.508 ms | 0.637 ms | 0.241 ms | 0.319 ms | 0.239 ms | 66 |
| `single_txn_medium_6col_10000` | 7.571 ms | 1.172 ms | 2.583 ms | 0.793 ms | 0.255 ms | 0.320 ms | 0.242 ms | 457 |
| `single_txn_large_10col_10000` | 9.700 ms | 5.188 ms | 4.907 ms | 0.785 ms | 0.241 ms | 0.315 ms | 0.238 ms | 2006 |
| `record_size_large_10col_10000` | 10.328 ms | 4.566 ms | 5.301 ms | 0.859 ms | 0.247 ms | 0.325 ms | 0.244 ms | 2006 |

Disposition: artifact-only profile. Do not treat this as a candidate result.
The next useful step is a direct profile of the transaction-strategy autocommit
and batched rows, because the existing insert profiler does not break down that
remaining gap.
