# DML Profile Refresh After Fresh-Eyes Verification

Date: 2026-05-17

Source state: `6b4181415c1e1a38c013b895cdca5f8ace522aaa` plus the current dirty
profiling/negative-ledger patch.

## Commands

```bash
FSQLITE_BENCH_PROFILE_DML=1 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 20 delete compare standard
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 30 delete compare standard
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 100 1000 update compare standard
FSQLITE_BENCH_PROFILE_DML=1 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 100 5 update compare standard
```

RCH had no admissible workers (`critical_pressure=6`) and failed open to local
execution.

## Results

| Workload | Mode | FrankenSQLite | C SQLite | Ratio |
|---|---:|---:|---:|---:|
| 10K rows / delete 500 rows | no profile | 563 ns/row | 374 ns/row | 1.51x |
| 10K rows / delete 500 rows | profiled | 1427 ns/row | 375 ns/row | 3.81x |
| 100 rows / update 10 rows | no profile | 782 ns/row | 417 ns/row | 1.87x |
| 100 rows / update 10 rows | profiled | 1629 ns/row | 477 ns/row | 3.41x |

The profiled 10K DELETE run confirms the same retained-leaf topology as the
earlier May 17 refresh:

- `direct_delete=500`, `slow=0`, `vdbe_opcodes=0`.
- Typical steady iterations show `delete_leaf_active=433/496`,
  `delete_leaf_miss=63`, and `delete_leaf_flush=64/64`.
- Representative steady counters are `delete_active_probe_ns` around
  `218-247 us`, `delete_leaf_flush_ns` around `68-88 us`,
  `delete_leaf_materialize` around `48-70 us`, and `delete_leaf_search`
  around `39-53 us`.
- Newly split fixed costs remain smaller than retained-leaf ceremony:
  `delete_preflush_ns` around `13 us`, `delete_rowid_ns` around `13 us`,
  `delete_memdb_abandon=500/12-14 us`, and
  `delete_memory_sync=500/13-15 us`.

The 100-row UPDATE row is already on the retained leaf-patch path:

- `direct_update=10`, `slow=0`, `update_leaf_start=1/1`,
  `update_leaf_active=9/9`, `update_leaf_flush=1/1`.
- Steady profiled iterations spend only about `4-5 us` in
  `execute_body_ns` and about `1.6-1.8 us` in `commit_roundtrip_ns`.

## Decision

No source optimization was attempted from this refresh. The surviving DML gap is
not a new micro-hotspot; it is the already-identified retained same-leaf
mutation ceremony plus page-image publication boundary.

The negative-results ledger already fences the tempting standalone patches in
this area: retained cursor shells, rowid search hints, scratch reset removal,
lazy UPDATE scratch borrowing, synced-root/MemDB invalidation trims, direct
flush/cursorless flush, dense rowid buffers, and affected-count-only logical
DELETE buffers.

The next source candidate remains the broader transaction-local row/key DML
mutation operator: prove affected counts at the mutation boundary, preserve
read-your-writes/rollback/savepoint semantics, group by B-tree leaf, and publish
each dirty leaf once through the existing MVCC/pager path. It should be kept
only if it improves the focused UPDATE/DELETE section and the full quick matrix.
