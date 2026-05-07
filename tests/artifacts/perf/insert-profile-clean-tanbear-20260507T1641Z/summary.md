# Clean insert hot-path profile

- Agent: TanBear
- Date: 2026-05-07
- Source: clean detached worktree at `977840591b56b9006b90158c8091529ba2d860a4`
- Baseline reference: `tests/artifacts/perf/full-quick-clean-tanbear-20260507T1633Z/summary.md`
- Target section: INSERT throughput rows that remain C-faster in the clean full quick matrix

## Commands

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-clean-perf-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-profile-clean-tanbear-20260507T1641Z/report.json --no-html
```

The parsed profile counters are in `profile-selected.tsv`.

## Matrix Read

This filtered/profiled run is diagnostic only. The authoritative full-matrix
baseline is `full-quick-clean-tanbear-20260507T1633Z`; enabling the profile hook
changes some tiny-row timings.

Filtered insert run:

- Total scenarios: `25`
- FrankenSQLite faster: `15`
- Comparable: `2`
- C SQLite faster: `8`
- Write-bulk geomean: `0.912820`
- Write-bulk p90: `1.160018`
- Write-bulk p99: `1.624609`

Top C-faster rows in this profiled run:

| Ratio | Section | Scenario | C median ms | F median ms |
| ---: | --- | --- | ---: | ---: |
| 1.624609 | Single Transaction large_10col | 100 rows | 0.152766 | 0.248185 |
| 1.398967 | Single Transaction medium_6col | 1000 rows | 0.543368 | 0.760154 |
| 1.160018 | Single Transaction medium_6col | 100 rows | 0.102682 | 0.119113 |
| 1.160008 | Single Transaction small_3col | 100 rows | 0.077265 | 0.089628 |
| 1.141532 | Record Size Comparison | large_10col 10K | 9.623803 | 10.985883 |
| 1.137842 | Transaction Strategy small_3col | 100 rows / single txn | 0.075151 | 0.085510 |
| 1.133839 | Transaction Strategy small_3col | 100 rows / batched (100/txn) | 0.076353 | 0.086572 |
| 1.062216 | Single Transaction medium_6col | 10000 rows | 5.631547 | 5.981922 |
| 1.021185 | Single Transaction tiny_1col | 100 rows | 0.069010 | 0.070472 |

## Hot-Path Read

Selected profile counters for the wide 10K rows:

| Label | rows | insert_us | commit_us | row_build_ns | btree_insert_ns | commit_roundtrip_ns | page_pool_misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `fs_insert_single_txn_medium_6col_10000` | 10000 | 7166.2 | 1518.5 | 2795783 | 344828 | 476642 | 457 |
| `fs_insert_single_txn_large_10col_10000` | 10000 | 9952.5 | 4279.5 | 5057682 | 851722 | 2056642 | 2006 |
| `fs_insert_record_size_medium_6col_10000` | 10000 | 6962.0 | 934.1 | 2652844 | 326357 | 334527 | 457 |
| `fs_insert_record_size_large_10col_10000` | 10000 | 9774.3 | 3692.3 | 5100094 | 664922 | 1812184 | 2006 |

The wide-row profile is still dominated by row construction and commit/page
volume, not by the B-tree mutation counter alone. For record-size `large_10col`
10K, `row_build_ns` is about `5.1 ms`, `btree_insert_ns` is about `0.66 ms`,
`commit_roundtrip_ns` is about `1.81 ms`, and page pool misses are `2006`.

## No-Retry Guardrail

Do not turn this profile into another standalone concat micro-patch. The
negative ledger already fences the obvious row-build variants, including:

- Param-one concat direct INSERT encoder.
- Text-piece concat collection/transducer.
- Direct INSERT concat owned-text move.
- Thread-local concat value pool.
- Concat pre-sizing scans and isolated `?1` formatting shortcuts.

The dirty local `crates/fsqlite-core/src/connection.rs` diff in the shared
worktree is exactly this rejected param-one concat family. It improved a focused
row-build metric but failed the full quick keep gate, so it should not be used
as a baseline or revived as a standalone optimization.

## Next Candidate Shape

The surviving shape from the ledger and this profile is a fused row-template
encoder coupled to a bulk/page-run writer, not a local concat-expression tweak.
Any retry should prove all of these before source changes are kept:

1. The exact large-row target improves in absolute FSQLite median.
2. `row_build_ns` drops without moving cost into `commit_us` or page-run replay.
3. The clean full quick matrix preserves the primary weighted score and does
   not add C-faster rows in small INSERT or UPDATE/DELETE.
