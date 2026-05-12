# Current DML Profile at 32b58eb

- Date: 2026-05-12 22:06 UTC.
- Commit: `32b58eb04b5e044ea6aa38341313d1ad45f39774`.
- Command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update --no-html`.
- Artifact: `update-delete-profile.json`.
- Worktree: clean; `benchmark_binary_older_than_git_head=false`.

## Rows

| Row | F ms | C ms | F/C |
|---|---:|---:|---:|
| 100 rows / update 10 rows | `0.005972` | `0.005340` | `1.1183520599x` |
| 100 rows / delete 5 rows | `0.006953` | `0.002324` | `2.9918244406x` |
| 1000 rows / update 100 rows | `0.028454` | `0.037190` | `0.7650981447x` |
| 1000 rows / delete 50 rows | `0.029695` | `0.015530` | `1.9121056021x` |
| 10000 rows / update 1000 rows | `0.246161` | `0.361386` | `0.6811580969x` |
| 10000 rows / delete 500 rows | `0.263043` | `0.158897` | `1.6554308766x` |

## DELETE Counters

Every profiled DELETE row stayed on the prepared direct path (`slow=0`).
For the 10K/500 DELETE row, the remaining cost is the retained-leaf ceremony:

- `delete_leaf_start=64/67`
- `delete_leaf_active=433/496`
- `delete_leaf_miss=63` (`60` out-of-leaf, `3` last-cell)
- `delete_leaf_flush=64/64`
- `delete_leaf_flush_ns=58259`
- `delete_leaf_materialize=64/42408`
- `delete_leaf_write=64/8497`
- `delete_leaf_search=560/40139`
- `delete_leaf_dupcheck=500/12492`
- `delete_leaf_compact=497/15597`
- `delete_leaf_cellparse=497/13005`

This reconfirms that another standalone retained-leaf micro-patch is the wrong
lever. The missing primitive is still a transaction-local DML mutation/read-view
operator that supplies affected-row, duplicate/missing-rowid, read-your-writes,
rollback/savepoint, schema-drift, and MVCC publication semantics together.
