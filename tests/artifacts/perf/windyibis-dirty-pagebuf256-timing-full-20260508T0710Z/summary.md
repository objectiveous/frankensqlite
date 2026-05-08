# Dirty PageBuf 256 + Timing Fix Full Quick Matrix

Date: 2026-05-08
Command:
`./.rch-target/release-perf/comprehensive-bench --quick --no-html --json-out tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-full-20260508T0710Z/full-quick.json`

This run measured `main` at `17b32ada9abbd477471347dc53b124c9f73353d8` with a dirty source tree owned by BoldLion:

- `crates/fsqlite-pager/src/page_buf.rs`: `GLOBAL_PAGE_BUF_RECYCLE_CAPACITY = 256`
- `crates/fsqlite-wal/src/group_commit.rs`: adds `commit_phase_timing_forced_enabled()`
- `crates/fsqlite-wal/src/lib.rs`: re-exports `commit_phase_timing_forced_enabled`
- `crates/fsqlite-e2e/src/fsqlite_executor.rs`: disabled metric captures preserve a forced commit-timing toggle

The benchmark binary was built from this dirty source before the artifact-only `17b32ada` commit, so the JSON reports `benchmark_binary_older_than_git_head=true`. The source mtimes predated the binary; treat this as dirty-integration evidence, not a clean committed-source gate.

## Result

| Metric | Dirty pagebuf256/timing | Prior keeper |
| --- | ---: | ---: |
| Primary weighted score | 0.3348866468 | 0.3358994390 |
| Average ratio | 0.4461065430 | 0.4420352711 |
| Geomean ratio | 0.2575107741 | 0.2593408377 |
| P90 ratio | 0.9757275559 | 1.0575874643 |
| P99 ratio | 1.4291042441 | 1.2422341250 |
| Faster / comparable / slower | 81 / 3 / 9 | 81 / 2 / 10 |

Lower ratios are better. The dirty integration is a small primary-score improvement over the prior keeper and improves p90, but it worsens p99 because the 100-row update row is now the worst tail row.

## Worst Remaining Rows

| Ratio | Category | Scenario | C SQLite | FrankenSQLite |
| ---: | --- | --- | ---: | ---: |
| 1.4291 | write_single | 100 rows / update 10 rows | 0.079899 ms | 0.114184 ms |
| 1.3893 | write_bulk | tiny_1col / 100 rows | 0.062557 ms | 0.086913 ms |
| 1.1390 | write_bulk | medium_6col / 100 rows | 0.097743 ms | 0.111329 ms |
| 1.1378 | write_bulk | 100 rows / batched (100/txn) | 0.071164 ms | 0.080972 ms |
| 1.1266 | concurrent_writers | 2 writers x 1000 rows | 13.281167 ms | 14.963211 ms |
| 1.1201 | write_bulk | 100 rows / single txn | 0.070793 ms | 0.079298 ms |
| 1.1137 | write_bulk | large_10col / 100 rows | 0.144280 ms | 0.160681 ms |
| 1.1105 | write_single | 100 rows / delete 5 rows | 0.108303 ms | 0.120275 ms |
| 1.0633 | write_bulk | large_10col / 10000 rows | 9.777459 ms | 10.396610 ms |

## Interpretation

Restoring the page buffer recycle cap to 256 removes the 2048-cap large-record regression seen in `windyibis-pagebuf-cap2048-20260508T062615Z`, where the large 10-column record-size row reached a 2.0421x ratio. In this dirty full matrix, the comparable 10K large-record row is 0.9724x in the record-size section and 1.0633x in the single-transaction large_10col section.

The next source decision belongs to the agent holding those file reservations. From this matrix alone, the dirty change is acceptable on the weighted gate, but any clean commit should be followed by a clean full quick run because the p99 tail moved in the wrong direction.
