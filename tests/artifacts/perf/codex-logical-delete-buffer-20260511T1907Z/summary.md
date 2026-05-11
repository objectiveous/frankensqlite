# Exact-MemDB Logical DELETE Buffer Screen

- Date: 2026-05-11
- Source commit inspected: `20a096808b6e`
- Target: remaining `UPDATE/DELETEThroughput` DELETE red rows, especially
  `100 rows / delete 5 rows`, `1000 rows / delete 50 rows`, and
  `10000 rows / delete 500 rows`.

## Candidate Screened

One apparent branch of the broader transaction-local DML mutation operator was
an exact-`MemDatabase` assisted logical DELETE buffer:

- answer duplicate/missing rowid checks from the exact `MemDatabase` table image,
- buffer rowid tombstones through the write-only transaction,
- flush physical B-tree deletes only at a read/savepoint/commit boundary.

This would remove per-row B-tree descent from the hot DELETE execute body only
if the benchmark connection already has an exact hydrated row mirror at the
start of the DELETE transaction.

## Evidence

The benchmark shape deliberately disables in-memory time-travel snapshots:

```text
crates/fsqlite-e2e/src/bin/comprehensive_bench.rs:513-520
FSQLITE_BENCHMARK_PRAGMAS includes:
PRAGMA fsqlite_capture_time_travel_snapshots=false;
```

The connection begin path only eagerly hydrates `:memory:` row images for
explicit BEGIN when time-travel capture is enabled:

```text
crates/fsqlite-core/src/connection.rs:11255-11259
should_hydrate_memdb_rows_for_explicit_begin() =
    should_eagerly_hydrate_memdb_rows()
    && (!pager.is_memory() || time_travel_capture_enabled)
```

Prepared direct INSERTs in private `:memory:` explicit transactions deliberately
keep the compatibility mirror lazy rather than carrying row images through the
write loop:

```text
crates/fsqlite-core/src/connection.rs:17522-17530
defer_lazy_memdb_materialization is true for pager.is_memory()

crates/fsqlite-core/src/connection.rs:17397-17421
the lazy path clears row scratch and calls abandon_exact_memdb_row_mirror()
```

The current DML profile confirms that DELETE execution in the matrix is already
on the prepared direct path without MemDB refresh work:

```text
tests/artifacts/perf/codex-next-dml-profile-20260511T1701Z/update-delete-profile.stderr
100 rows / delete 5 rows: direct_delete=5, slow=0, memdb_refresh=0
1000 rows / delete 50 rows: direct_delete=50, slow=0, memdb_refresh=0
10000 rows / delete 500 rows: direct_delete=500, slow=0, memdb_refresh=0
```

The current full quick matrix remains:

```text
tests/artifacts/perf/codex-delete-run-borrow-flush-20260511T1609Z/full-quick-final-local.json
100 rows / delete 5 rows: F/C 2.838x
1000 rows / delete 50 rows: F/C 1.829x
10000 rows / delete 500 rows: F/C 1.595x
```

## Conclusion

No source patch was attempted. An exact-MemDB logical DELETE buffer is not a
viable standalone slice for the current benchmark rows because the exact row
mirror is intentionally absent in the measured write-only `:memory:` shape.
Forcing hydration would add an O(existing rows) setup/read-boundary cost before
the DELETE transaction, which is the class of MemDB work the current benchmark
pragmas and direct-DML path are designed to avoid.

The next viable DML operator cannot depend on a hydrated `MemDatabase` row
mirror for membership. It needs either a separate exact rowid membership
manifest maintained by the storage mutation path, or a deeper B-tree/MVCC
mutation representation that can prove duplicate/missing rowid, read boundary,
rollback/savepoint, and publication semantics without rehydrating row values.
