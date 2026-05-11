# DML Mutation Frontier Recertification

Date: 2026-05-11

Current `HEAD`: `94ebb38c33508d374c157c47f1af0df2f3bec3ff`

## Purpose

Re-read the current DML DELETE frontier after the latest profile/artifact
commits and decide whether a source patch is justified. This pass intentionally
screened for an unfenced one-lever optimization before editing source.

## Evidence Read

- `tests/artifacts/perf/codex-delete-run-borrow-flush-20260511T1609Z/full-quick-final-local.json`
  remains the latest full quick keep artifact for the retained delete-run win.
  Its `UPDATE/DELETEThroughput` DELETE rows are still red: `2.838x` for
  5 deletes, `1.829x` for 50 deletes, and `1.595x` for 500 deletes.
- `tests/artifacts/perf/codex-next-dml-profile-20260511T1701Z/summary.md`
  shows every profiled DELETE stays on the prepared direct path. The 10K/500
  row records 433 retained same-leaf hits across 496 attempts, 63 leaf-boundary
  misses, 64 dirty flushes, about `73.5 us` of materialization, and about
  `7.5 us` of page-write time.
- `tests/artifacts/perf/codex-current-delete-cpu-profile-20260511T1745Z/summary.md`
  refreshes CPU-symbol evidence and still points to the known
  `TransactionKind::get_page`, `TableLeafDeleteRun::delete_rowid_with_reason`,
  `TransactionKind::write_page_data`, and `TransactionKind::free_page`
  families.
- `tests/artifacts/perf/codex-concurrent-profile-json-20260511T1825Z/concurrent.json`
  confirms the low-thread concurrent gap is not a direct INSERT page-run miss:
  the rows have `page_run_flushes=0`, `slow=0`, and active direct-insert fast
  lanes.

## Fresh-Eyes Screen

The fresh source read covered:

- `SimpleTransaction::get_page`, `write_page_data`, `free_page`, normal commit,
  and retained commit in `crates/fsqlite-pager/src/pager.rs`.
- Prepared direct DELETE buffering and flush boundaries in
  `crates/fsqlite-core/src/connection.rs`.
- `TableLeafDeleteRun` search/materialization and cursor flush in
  `crates/fsqlite-btree/src/cursor.rs`.
- The existing B-epsilon tree/message-buffer implementation in
  `crates/fsqlite-btree/src/be_tree.rs`.
- The current optimization card in
  `docs/design/profile-first-optimization-cards-and-proof-packs.md`.

No new narrow source lever survived the ledger screen. The tempting edits are
already fenced by same-window benchmark rejects: direct writer/cursorless
delete-run flush, freed-page lookup variants, leaf-run admission and search
hints, materializer thresholds, private-memory page-1 or commit shortcuts,
explicit `commit_and_retain` deferral, no-op direct-write flush pre-gates, and
tombstone-only overlays.

## Alien Candidate Routing

The bounded alien-graveyard scan mapped the live symptom to B-epsilon style
message buffering, LeanStore/pointer-swizzling page-access reductions, and
parallel WAL. The current DELETE rows are private-memory DML rows, not WAL I/O
rows, so the actionable candidate remains the already-open transaction-local
DML mutation operator: buffer logical rowid delete/update messages in key space,
merge them at proven observation boundaries, and publish the final page/MVCC
surface as a batch.

## Decision

No source patch was attempted in this pass.

The next source attempt should be the broader transaction-local DML mutation
operator in `docs/design/profile-first-optimization-cards-and-proof-packs.md`,
not another retained leaf-run or pager micro-patch. The keep gate remains:
focused `--quick --filter update` wins on 5-row, 50-row, and 500-row DELETE
without UPDATE regression, followed by full quick primary-score neutrality or
better.
