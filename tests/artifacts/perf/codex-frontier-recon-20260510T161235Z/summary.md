# Frontier Recon Blocker

Date: 2026-05-10

Current HEAD reviewed: `b1a75085 docs(perf): reject bulk root fit probe`.

Purpose: fresh-eyes pass over the remaining full-quick C-faster frontier after
the DML refresh, non-DML rescreen, concurrent profile hook, multi-leaf DELETE
candidate rejection, and empty-root bulk-load root-fit candidate rejection.

Evidence reviewed:

- Full quick source of truth:
  `tests/artifacts/perf/codex-fresh-frontier-full-quick-20260510T093306Z/full-quick.json`.
  It reports `93` scenarios, `79 / 1 / 13`
  FrankenSQLite faster / comparable / C SQLite faster, geomean
  `0.273684916`, weighted primary score `0.375311640`, and p99 ratio
  `3.187473904`.
- DML head profile:
  `tests/artifacts/perf/codex-dml-head-profile-20260510T144411Z/summary.md`.
  It shows the DELETE tail as many leaf-run flushes at out-of-leaf boundaries,
  not an isolated cell-delete primitive.
- Non-DML rescreen:
  `tests/artifacts/perf/codex-nondml-frontier-rescreen-20260510T1506Z/summary.md`.
  It finds no remaining safe parser/setup/concat/direct-page-run microfamily.
- Concurrent profile hook:
  `tests/artifacts/perf/codex-concurrent-profile-hook-20260510T1140Z/stderr.log`.
  The profiled 2/4/8 writer shared-table rows stay on the prepared direct
  INSERT fast path (`fast == direct_insert`, `slow == 0`) while file-backed
  page runs remain inactive (`page_run_flushes=0`). The visible cost is page
  lock waits and stale-snapshot transaction retries: `mvcc_stale_snapshot`
  `12`, `72`, and `318` for 2/4/8 writers respectively.
- Negative-results ledger: `docs/progress/perf-negative-results.md`, especially
  the 2026-05-10 entries for non-DML rescreen, DML head refresh, file-backed
  concurrent INSERT page-run, multi-leaf DELETE backlog, and empty-root
  bulk-load root-fit.

Code surfaces reread:

- `crates/fsqlite-vdbe/src/engine.rs`: stale-snapshot first-touch checks and
  page-lock wait paths reject transactions when the commit index has advanced
  past the transaction snapshot for the touched page. That path is conflict
  policy, not a harness retry bug.
- `crates/fsqlite-core/src/connection.rs`: `BEGIN CONCURRENT` snapshot binding,
  commit conflict-page collection, prepared direct INSERT flushing, and
  page-run admission. The existing page-run fast path is memory-only; widening
  it for file-backed concurrent INSERT as a standalone patch is already a
  measured negative.
- `crates/fsqlite-mvcc/src/begin_concurrent.rs`: first-committer-wins conflict
  validation is driven by tracked write-conflict pages and the commit index.
- `crates/fsqlite-pager/src/pager.rs`: WAL conflict-page prediction already
  excludes synthetic page-one metadata from concurrent conflict surfaces unless
  page 1 is explicitly dirty.
- `crates/fsqlite-btree/src/cursor.rs`: bulk empty-root and depth-2 right-edge
  append helpers exist and are used by the current memory INSERT page-run lane;
  the root-fit shortcut candidate was measured and reverted.

Rejected no-edit candidates:

- Per-statement retry in the concurrent harness. A stale snapshot poisons the
  current transaction; the correct retry unit is the whole `BEGIN` through
  `COMMIT` workload attempt.
- Page-one conflict-surface trimming. The pager already avoids synthetic page 1
  for WAL concurrent commits, and the concurrent profile reports
  `mvcc_page_one_tracks=0`.
- File-backed concurrent page-run admission as a standalone source patch. The
  adjacent profile and ledger show this needs a fused page-construction and
  MVCC-publication design, not an admission guard flip.
- Standalone row-build/template/concat/setup trimming. These families are
  already covered by prior full-quick rejections and the non-DML rescreen.
- Wait-slice or stale-snapshot policy tuning. That would change conflict
  topology behavior without fixing the underlying shared-page publication
  representation.
- Linear retained multi-leaf DELETE backlogs. The candidate was correctness
  proofed, benchmarked, and rejected.

Next credible implementation boundary:

- Non-DML/concurrent: a fused record/page builder that computes row bodies and
  B-tree page layout together, then publishes the resulting pages through the
  pager/MVCC path. For file-backed `BEGIN CONCURRENT`, the design must batch
  page construction and MVCC page publication together and prove it improves 2/4
  writer rows without regressing 8 writers.
- DML: a true transaction-level many-leaf mutation representation with
  read-your-writes semantics, savepoint/rollback behavior, MVCC publication
  proof tests, a focused UPDATE/DELETE A/B win, and a full quick primary-score
  win in the same measurement window.

Keep gate for either path:

- Focused workload A/B must move the target rows and not merely a nearby metric.
- Full quick must improve or preserve the weighted primary score and C-faster
  row count in the same measurement window.
- Any rejected candidate must be reverted and entered into
  `docs/progress/perf-negative-results.md`.
