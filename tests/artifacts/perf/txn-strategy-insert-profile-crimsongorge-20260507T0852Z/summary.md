# Transaction-Strategy INSERT Profile

- Agent: CrimsonGorge
- Commit: `0f6a2fd67c8ae578e5616922b27fe3d70666959c`
- Worktree: `/data/tmp/frankensqlite-txn-profile-crimsongorge-20260507T0845Z`
- Build target: `/data/tmp/frankensqlite-crimsongorge-main-release`
- Command: `FSQLITE_BENCH_PROFILE_INSERT=1 comprehensive-bench --quick --filter transaction --json-out report-transaction.json --no-html`

## Result

The new Section 2 profile confirms the remaining `small_3col` gap is transaction-bound direct INSERT overhead, not parser or WAL cost.

| Row | C SQLite | FrankenSQLite | Ratio | Main profile signal |
| --- | ---: | ---: | ---: | --- |
| 100 rows / autocommit | 130.0 us | 170.5 us | 1.31x slower | per-row implicit txn dominates fixed cost |
| 1000 rows / autocommit | 847.6 us | 1.11 ms | 1.31x slower | `autocommit_begin_ns=176675`, `autocommit_resolve_ns=245154`, `cursor_setup_ns=238315` |
| 10000 rows / autocommit | 8.35 ms | 11.58 ms | 1.39x slower | `autocommit_begin_ns=1640652`, `autocommit_resolve_ns=2686682`, `cursor_setup_ns=2356623`, `btree_insert_ns=3650223` |
| 10000 rows / batched (1000/txn) | 3.23 ms | 4.25 ms | 1.31x slower | explicit BEGIN+COMMIT totals only about 0.22 ms; loss is direct/btree work |
| 10000 rows / single txn | 3.45 ms | 2.98 ms | 1.16x faster | single transaction remains a keep; do not optimize it at the expense of the matrix |

## Interpretation

- Autocommit pays a per-row implicit transaction tax: `autocommit_begin_ns + autocommit_resolve_ns` is about 4.33 ms at 10K rows.
- Autocommit also stays on the full-cell assembly append shape: `btree_cell_assembly_calls=10000` and `btree_leaf_full_cell_appends=9937`.
- Batched 1000/txn does not lose primarily on explicit transaction calls: profiled explicit `begin_us + commit_us` is about 0.22 ms at 10K rows.
- Batched loses after the first transaction because the empty-root page-run/bulk-load shape no longer applies; it falls back to repeated right-edge direct/btree work across later batches.
- The likely high-EV `connection.rs` seam is preserving or recreating a page-run/right-edge append plan across explicit memory batches, or using the existing btree writer append APIs from the direct lane so autocommit avoids full-cell assembly.

`crates/fsqlite-core/src/connection.rs` was exclusively reserved by PurpleOtter while this artifact was produced, so this run records the evidence without editing that source file.
