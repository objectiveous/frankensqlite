# DML Live Operator Boundary - 2026-05-17

## Scope

This artifact records the next implementation boundary for the remaining
prepared direct DELETE gap. It intentionally does not introduce a source patch:
the current evidence fences the obvious one-file and one-helper changes.

Source reviewed: `main @ 5dd371f74781`.

## Evidence Inputs

- Current focused DML boundary:
  `tests/artifacts/perf/codex-dml-operator-boundary-head-20260517Tnext/summary.md`
- Post-attribution focused DML profile:
  `tests/artifacts/perf/codex-dml-profile-after-active-probe-fix-20260517T1730Z/summary.md`
- Current full-quick frontier:
  `tests/artifacts/perf/codex-current-fullquick-frontier-20260517T1900Z/summary.md`
- Negative ledger:
  `docs/progress/perf-negative-results.md`
- Existing operator cards:
  `docs/design/profile-first-optimization-cards-and-proof-packs.md` section 8.4,
  `tests/artifacts/perf/codex-dml-logical-mutation-boundary-20260512T100623Z/summary.md`,
  and
  `tests/artifacts/perf/codex-dml-operator-probe-20260513T0303Z/summary.md`.

`br list --json` still reports a malformed Beads database image, but
`bv --robot-triage` can read the JSONL graph. Its relevant ready signals are
the `bd-db300.8.2` and `bd-db300.5.2` concurrency/performance areas. The
benchmark artifacts above remain the authoritative evidence for this pass.

## Current Matrix Fact

The stable red path is still `write_single` prepared direct DELETE:

| Scenario | Stable current signal |
| --- | --- |
| `100 rows / delete 5 rows` | red, noisy in the latest boundary run |
| `1000 rows / delete 50 rows` | red across focused and full-quick runs |
| `10000 rows / delete 500 rows` | red across focused and full-quick runs |

The post-attribution profile shows the DELETE cost is distributed across:

- active retained-run probing,
- same-leaf run materialization and flush,
- leaf search,
- duplicate checks,
- cell parsing,
- per-row MemDatabase/QF/count-cache maintenance,
- ordinary cursor seek and physical delete fallbacks.

That distribution is the important result. No single helper owns enough stable
cost to justify another micro-patch.

## SQLite Reference Path

The C SQLite path is still a cursor-positioned local mutation:

- `legacy_sqlite_code/sqlite/src/vdbe.c` `OP_Delete` calls
  `sqlite3BtreeDelete(pC->uc.pCursor, pOp->p5)`.
- `legacy_sqlite_code/sqlite/src/btree.c` `sqlite3BtreeDelete` restores a
  valid cursor, computes cell info, writes the page, clears overflow if any,
  calls `dropCell`, and balances only when the page crosses the threshold.
- `dropCell` removes the cell pointer, frees cell content space, and shifts the
  pointer array locally.
- `balance` exits early when no overflow exists and free space is under the
  two-thirds threshold.

For the benchmark shape, C SQLite pays cursor-positioned delete work and avoids
FrankenSQLite's transaction-level page-publication ceremony on every row.

## FrankenSQLite Live Path

The current Rust path is a page-image path with a same-leaf retention helper:

- `crates/fsqlite-core/src/connection.rs`:
  `execute_prepared_direct_simple_delete` flushes pending update/insert runs,
  tries an active same-leaf delete run, then creates a B-tree cursor and falls
  back to rowid seek plus physical delete.
- `execute_prepared_direct_simple_delete_with_cursor` proves affected-row
  semantics with `table_move_to`, optionally creates a `TableLeafDeleteRun`,
  otherwise calls `cursor.delete`.
- `try_execute_prepared_direct_simple_delete_active_leaf_run` probes the active
  retained leaf run, stages completed monotone leaves, and flushes when the
  next row cannot be admitted.
- `crates/fsqlite-btree/src/cursor.rs`: `TableLeafDeleteRun` owns one cloned
  table-leaf image, searches that leaf for each rowid, records deleted cell
  indices, materializes the leaf image, and publishes it through
  `write_page_data`.
- `crates/fsqlite-pager/src/traits.rs`: `TransactionHandle` exposes
  page-level operations (`get_page`, `write_page`, `write_page_data`,
  `allocate_page`, `free_page`, commit/savepoint/rollback state). It does not
  expose a rowid/cell mutation log.
- `crates/fsqlite-vdbe/src/engine.rs`: `SharedTxnPageIo` wraps a pager
  `TransactionKind`; B-tree page reads resolve through `TransactionHandle`.
- `crates/fsqlite-mvcc/src/lifecycle.rs`: `MvccManager` already has
  `CellVisibilityLog` and `read_page_with_cell_deltas`, but that primitive is
  not wired into the live core/VDBE/pager path used by this benchmark.

This means the representation boundary, not the leaf helper itself, is now the
dominant design problem.

## Rejected Local Levers

Do not retry these as standalone changes unless a newer benchmark invalidates
the current ledger:

- retained-run active probe tweaks,
- leaf search and cursor-resume hints,
- duplicate-check or cell-parse micro-optimizations,
- same-leaf materialization rewrites,
- synced-root or MemDatabase invalidation tweaks,
- exact-MemDB rowid queues,
- dense rowid/tombstone buffers,
- no-retry harness rewiring,
- physical retained-leaf backlog variants.

Each item either failed the focused matrix already or targets a cost bucket too
small and distributed to clear the keep gate alone.

## Graveyard Mapping

The applicable buried-CS primitive is a B-epsilon/message-buffer style write
path, compiled down to a transaction-local rowid/key mutation operator:

- `alien_cs_graveyard.md` maps write-optimized indexes and message buffers to
  database write-amplification problems.
- The FrankenSuite summary maps FrankenSQLite to page-level MVCC, B-epsilon
  trees for WAL/index write amplification, parallel WAL, flat combining, and
  version-chain metadata/conflict-detection artifacts.
- For this exact red path, B-epsilon is the best fit because the repeated
  operations are logical rowid deletes in one transaction, while the current
  implementation eagerly re-enters physical leaf/page machinery.
- RLU-style writer logs are a useful mental model for read-your-writes:
  readers must consult the transaction's pending logical messages before the
  base page image.
- Semi-naive delta propagation is relevant for invalidation boundaries: QF,
  count/sum caches, MemDatabase mirrors, and MVCC witnesses should consume the
  delta set once, not re-run per-row maintenance after every row.

## Recommendation Contract

Change:
Introduce a transaction-local DML mutation operator that stores logical rowid
messages keyed by `(root_page, rowid)` and publishes or materializes them at a
proven read/commit boundary.

Hotspot evidence:
Prepared direct DELETE remains the durable red path in the current focused DML
and full-quick artifacts. The post-attribution profile shows distributed
per-row retained-run ceremony rather than one isolated helper.

Mapped graveyard sections:
B-epsilon/message-buffered write path, RLU-style writer log/read overlay,
semi-naive delta propagation, and FrankenSuite's FrankenSQLite MVCC/SSI
version-chain artifact map.

EV score:
Impact 5 * confidence 3 / effort 4 = 3.75. This remains above the
implementation threshold; all smaller candidates score at or below 1.0.

Priority:
A for the DELETE red path. S only after the read-view contract has focused
correctness tests, because a partial overlay can silently under-delete.

Adoption wedge:
Start with the benchmark-relevant safe shape only: private `:memory:`,
explicit transaction, prepared direct-simple `DELETE FROM table WHERE id = ?1`,
no savepoints, no internal statement savepoint, no triggers, no foreign keys,
no indexes, no virtual tables, no RETURNING, and no retained count/sum cache.

Budgeted mode:
Bound the pending mutation set by row count and distinct page estimate. On
budget exhaustion or unsupported observation, materialize through the existing
physical path and clear the logical set.

Expected-loss model:
False positive activation costs performance; false negative visibility or
affected-row proof corrupts SQL semantics. The operator must prefer flushing to
guessing.

Fallback trigger:
Any read path that cannot consult the logical mutation view must flush first.
SAVEPOINT/ROLLBACK paths must either own mutation-log pruning or force a flush
before the savepoint is accepted.

Isomorphism proof plan:
Compare against C SQLite/rusqlite for affected-row counts, duplicate deletes,
missing rowids, read-after-delete, scan-after-delete, rollback, savepoint,
commit, schema drift, count/sum cache behavior, and concurrent-mode defaults.

Before/after target:
Focused `--quick --filter update-delete` should improve all three DELETE
medians without regressing medium/large UPDATE. Full quick must be weighted
score neutral or better with no new critical red rows.

Primary risk:
Read-your-writes drift. Countermeasure: all point reads, scans, row counts,
QF/count-cache paths, MemDatabase reloads, and VDBE fallback boundaries must
either consult the overlay or flush the operator.

Rollback:
Revert the single implementation commit. Do not leave a permanent
off-by-default compatibility shim.

Baseline comparator:
C SQLite on the same `comprehensive-bench --quick --filter update-delete`
workload and the current `main` focused/full-quick artifacts above.

## First Source Slice

The next source patch should not start in `TableLeafDeleteRun`. It should start
by defining the live-path integration boundary:

1. Add a transaction-owned logical DML message store keyed by stable rowid/key
   identity, not physical cell index.
2. Add a read-view adapter so B-tree point lookup and table scan can overlay
   this transaction's own messages before returning base page state.
3. Thread rollback/savepoint ownership through the same lifecycle as staged
   page writes.
4. Publish conflict/witness information through the same page/cell surface used
   by concurrent writers.
5. Keep the first activation shape narrow and fail closed to the existing
   physical delete path.

Until those five pieces exist, another physical same-leaf optimization is the
wrong layer.
