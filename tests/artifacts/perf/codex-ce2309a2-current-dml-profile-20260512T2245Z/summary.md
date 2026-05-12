# Current DML Profile at ce2309a2

- Date: 2026-05-12 22:44 UTC.
- Commit: `ce2309a2d7c3bbfcfd82340276933d53725051df`.
- Command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update --no-html`.
- Artifact: `update-delete-profile.json`.
- Worktree: clean; `benchmark_binary_older_than_git_head=false`.

## Rows

| Row | F ms | C ms | F/C |
|---|---:|---:|---:|
| 100 rows / update 10 rows | `0.006061` | `0.004468` | `1.3565353626x` |
| 100 rows / delete 5 rows | `0.007374` | `0.002385` | `3.0918238994x` |
| 1000 rows / update 100 rows | `0.028764` | `0.038873` | `0.7399480359x` |
| 1000 rows / delete 50 rows | `0.029195` | `0.015679` | `1.8620447733x` |
| 10000 rows / update 1000 rows | `0.244598` | `0.372427` | `0.6567676350x` |
| 10000 rows / delete 500 rows | `0.261950` | `0.158417` | `1.6535472834x` |

## DELETE Counters

Every profiled DELETE row stayed on the prepared direct path (`slow=0`).
For the 10K/500 DELETE row, the remaining cost is the retained-leaf ceremony:

- `delete_leaf_start=64/67`
- `delete_leaf_active=433/496`
- `delete_leaf_miss=63` (`60` out-of-leaf, `3` last-cell)
- `delete_leaf_flush=64/64`
- `delete_leaf_flush_ns=52538`
- `delete_leaf_materialize=64/39847`
- `delete_leaf_write=64/7245`
- `delete_leaf_search=560/40230`
- `delete_leaf_dupcheck=500/12366`
- `delete_leaf_compact=497/15628`
- `delete_leaf_cellparse=497/12977`

This reconfirms that another standalone retained-leaf micro-patch is the wrong
lever. The missing primitive is still a transaction-local DML mutation/read-view
operator that supplies affected-row, duplicate/missing-rowid, read-your-writes,
rollback/savepoint, schema-drift, and MVCC publication semantics together.
