# Full Quick Matrix Refresh - 2026-05-07

Agent: CrimsonGorge

Baseline: clean release-perf binary built from git
`5b36871d3fb29b9728ec9e34c64a8c21b45151f1` in
`/data/tmp/frankensqlite-concurrent-profile-crimsongorge-20260507T123150Z`.
The shared worktree had unrelated dirty `connection.rs` changes when this
artifact was published; they were not part of the measured binary.

Command:

```bash
/data/tmp/frankensqlite-concurrent-profile-target/release-perf/comprehensive-bench \
  --quick \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/full-refresh-crimsongorge-20260507T1246Z/report-full.json \
  --no-html
```

## Result

- Total scenarios: 93.
- FrankenSQLite faster: 74.
- Comparable: 8.
- C SQLite faster: 11.
- Primary metric: `per_category_weighted.score`.
- Primary weighted score: `0.3626703007868455`.
- Geomean ratio: `0.27513970727801706`.
- Median ratio: `0.29682001407639647`.
- `p90_ratio`: `1.125526097508525`.
- `p99_ratio`: `1.6637239463862816`.

## Remaining C-Faster Rows

| Ratio | Section | Scenario | C SQLite ms | FrankenSQLite ms | Category |
| ---: | --- | --- | ---: | ---: | --- |
| 1.6637 | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.080651 | 0.134181 | write_single |
| 1.5415 | UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.084478 | 0.130224 | write_single |
| 1.3763 | INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.150892 | 0.207679 | write_bulk |
| 1.2771 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 10000 rows / batched (1000/txn) | 3.352492 | 4.281322 | write_bulk |
| 1.2265 | INSERTThroughput - Single Transaction - medium_6col | 1000 rows | 0.613109 | 0.751949 | write_bulk |
| 1.1876 | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.078437 | 0.093154 | write_bulk |
| 1.1536 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 12.113046 | 13.973212 | concurrent_writers |
| 1.1518 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.082284 | 0.094778 | write_bulk |
| 1.1331 | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B) | 9.679507 | 10.967841 | write_bulk |
| 1.1255 | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | 0.390612 | 0.439644 | write_single |
| 1.0954 | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 0.365304 | 0.400160 | write_single |
| 1.0337 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 1000 rows / batched (1000/txn) | 0.354504 | 0.366466 | write_bulk |
| 1.0259 | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.070061 | 0.071875 | write_bulk |
| 1.0139 | INSERTThroughput - Single Transaction - large_10col | 1000 rows | 0.911498 | 0.924202 | write_bulk |
| 1.0078 | INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.114775 | 0.115667 | write_bulk |

## Diagnosis

The residual gap is no longer broad storage, WAL, or MVCC overhead. The remaining
C-faster rows cluster in two places:

1. Small-row direct UPDATE/DELETE, where several standalone ideas have already
   been rejected in the negative-results ledger because removing one small cost
   at a time failed the matrix keep gate.
2. Direct INSERT page construction and row materialization, especially small
   single-transaction rows, large-record record building, and the non-empty-root
   batched append row.

The highest-EV next source seam remains a true fused direct-DML cursor/page-run
or non-empty-root right-edge page builder in `crates/fsqlite-core/src/connection.rs`,
with B-tree support only if the design avoids row-at-a-time full-cell replay.
The ledger rules out retrying standalone record-layout reuse, fixed-width
payload patching, scratch-reset trimming, schema-proof carry, and row-at-a-time
non-empty page-run buffering.
