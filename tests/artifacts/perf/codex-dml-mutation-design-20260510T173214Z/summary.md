# DML Mutation Design Blocker

Date: 2026-05-10

Current HEAD reviewed: `238eda5d docs(perf): reject delete leaf next-cell hint`.

## Trigger

This pass followed three same-session rejected micro-candidates:

- `tests/artifacts/perf/codex-fused-empty-root-pagerun-20260510T165017Z/summary.md`
- `tests/artifacts/perf/codex-disable-delete-leaf-run-20260510T170227Z/summary.md`
- `tests/artifacts/perf/codex-delete-leaf-next-hint-20260510T171758Z/summary.md`

It also rechecked the current frontier blocker at
`tests/artifacts/perf/codex-frontier-recon-20260510T161235Z/summary.md`, the
DML head refresh at
`tests/artifacts/perf/codex-dml-head-profile-20260510T144411Z/summary.md`, and
the top of `docs/progress/perf-negative-results.md`.

## Fresh Read

Source seams reread:

- `crates/fsqlite-core/src/connection.rs:18144`:
  `execute_prepared_direct_simple_delete` flushes pending UPDATE and INSERT
  runs, tries the currently retained DELETE leaf run, then builds a fresh cursor
  for the rowid fallback.
- `crates/fsqlite-core/src/connection.rs:18238`:
  `execute_prepared_direct_simple_delete_with_cursor` must seek by rowid before
  it can create a same-leaf delete run, and it falls back to ordinary
  `cursor.delete` when the retained run cannot accept the row.
- `crates/fsqlite-core/src/connection.rs:31695`:
  `flush_pending_direct_delete_leaf_run` materializes one retained dirty leaf by
  constructing a cursor over the active transaction or concurrent page I/O.
- `crates/fsqlite-core/src/connection.rs:31789`:
  `flush_pending_direct_write_runs` is the shared read/savepoint/commit
  observation boundary for pending UPDATE, DELETE, and INSERT runs.
- `crates/fsqlite-core/src/connection.rs:31872`:
  current DELETE deferral is intentionally limited to private memory, explicit
  transactions, no savepoints, no internal statement savepoint, and no retained
  count/sum cache.
- `crates/fsqlite-core/src/connection.rs:31915`:
  active DELETE run misses immediately flush the run and re-enter the ordinary
  rowid seek path.
- `crates/fsqlite-core/src/connection.rs:39180` and
  `crates/fsqlite-core/src/connection.rs:39203`:
  rollback-to-savepoint/full-rollback paths invalidate direct-write side state
  and quotient filters.
- `crates/fsqlite-core/src/connection.rs:39266` and
  `crates/fsqlite-core/src/connection.rs:39431`:
  SAVEPOINT and RELEASE are hard flush boundaries for pending direct writes.
- `crates/fsqlite-btree/src/cursor.rs:4660`:
  INSERT already has an empty-root sorted-record bulk builder; the rejected
  fused page-run candidate showed that moving page image construction into the
  per-row execution path is not a keepable standalone design.
- `legacy_sqlite_code/sqlite/src/vdbe.c:5901`,
  `legacy_sqlite_code/sqlite/src/btree.c:9826`, and
  `legacy_sqlite_code/sqlite/src/btree.c:7252`:
  the C SQLite comparison remains cursor-positioned delete plus local
  `dropCell` pointer movement/free-space accounting. It does not pay a
  transaction-local MVCC publication ceremony per row.

## Conclusion

No safe one-lever source patch remains in this surface.

The current retained DELETE helper owns one leaf-local mutation run. Extending
it with another small admission or seek hint is fenced by the latest rejected
candidate. Extending it into a scanned list of retained leaves is fenced by the
multi-leaf backlog rejection. Disabling it is fenced by the delete-run disable
probe. The remaining gap is the representation boundary: many rowid mutations
inside one transaction need to be represented as a transaction-level mutation
set and published through pager/MVCC once, rather than as independent
leaf-local cursor episodes.

## Required Shape For The Next Source Patch

A keepable DML patch needs a transaction-level many-leaf mutation representation
with these properties:

1. Admit only the benchmark-relevant safe shape first: private `:memory:`,
   explicit transaction, direct-simple rowid DELETE, no savepoints, no internal
   statement savepoint, no triggers, no foreign keys, no RETURNING, no indexes,
   no retained count/sum cache.
2. Store rowid tombstones or leaf deltas in an order-preserving transaction
   structure keyed by table root and rowid. It must avoid per-row scans of prior
   leaves; the linear backlog shape already lost.
3. Preserve read-your-writes. Before any read boundary, table scan, point seek,
   MemDatabase reload, count/sum cache consult, or statement that cannot
   consult the mutation set, the patch must either materialize the mutation set
   or route that observation through it.
4. Treat SAVEPOINT, RELEASE, ROLLBACK TO, full ROLLBACK, failed flush, and
   missing active transaction as proof obligations. The current tests around
   pending insert/delete runs show the minimum rollback/flush preservation bar.
5. Preserve quotient-filter and retained mirror invalidation semantics. A
   mutation overlay that forgets rollback invalidation can under-delete later.
6. Publish through pager/MVCC as a batch at materialization or commit. If the
   implementation still performs one cursor seek and one page publication per
   row, it has not crossed the blocker.

## Proof Gate

The next implementation should not be merged unless it passes all of these in
the same A/B window:

- Focused correctness tests for direct DELETE read-your-writes, commit,
  rollback, failed flush preservation, SAVEPOINT boundary, and ROLLBACK TO.
- `cargo check -p fsqlite-core -p fsqlite-btree --all-targets`.
- `cargo clippy -p fsqlite-core -p fsqlite-btree --all-targets -- -D warnings`.
- Focused `comprehensive-bench --quick --filter update-delete` must improve the
  DELETE target rows, especially `1000 rows / delete 50 rows` and
  `10000 rows / delete 500 rows`, without giving back the retained same-leaf
  win.
- A same-window full quick run must improve or preserve the weighted primary
  score and C-faster row count.

## Non-Goals

Do not retry these as standalone follow-ups from this artifact:

- another same-leaf DELETE run admission tweak;
- next-cell or retained-cursor seek hints;
- linear scanned backlogs of dirty leaf runs;
- disabling the retained DELETE run;
- standalone row serializer/template/page-run changes;
- standalone file-backed concurrent page-run admission;
- conflict-policy or wait-slice tuning.
