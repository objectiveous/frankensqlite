# Frontier Boundary Rerank

Date: 2026-05-10T22:13:43Z
HEAD: `abe56839`

## Purpose

This pass rechecked the current performance frontier after the rejected direct
DELETE stack-entry detach slice. The goal was to avoid repeating another
standalone micro-optimization that the same-window benchmark matrix has already
ruled out.

## Inputs Reviewed

- Current red rows:
  `tests/artifacts/perf/codex-current-full-quick-20260510T182554Z/full-quick.json`.
- INSERT repeat/profile screen:
  `tests/artifacts/perf/codex-insert-red-repeat-20260510T183336Z/summary.md`.
- Low-thread concurrent repeat/profile screen:
  `tests/artifacts/perf/codex-concurrent-repeat-after-63cf-20260510T181706Z/`.
- Non-DML frontier rescreen:
  `tests/artifacts/perf/codex-nondml-frontier-rescreen-20260510T1506Z/summary.md`.
- DML head profile:
  `tests/artifacts/perf/codex-dml-head-profile-20260510T144411Z/summary.md`.
- Scratch rejection sync:
  `tests/artifacts/perf/codex-frontier-scratch-rejection-sync-20260510T1740Z/summary.md`.
- 16-thread shared-table verification:
  `tests/artifacts/perf/codex-shared16-mtmvcc-20260510T195814Z/summary.md`.
- Alien-graveyard routing for FrankenSQLite candidates:
  B-epsilon trees, latch-free MVCC, flat combining, and parallel WAL were
  considered only at the representation-boundary level.

## Current Boundary

The README no longer contains the stale "16-thread shared-table
BUSY_SNAPSHOT storm" note. The retained verification artifact reports
`16` shared-table writer threads with `0` FSQLite failures and `9.80x`
throughput versus C SQLite in that run.

The remaining C-faster rows are not explained by a single local hot branch:

- INSERT is already on the prepared direct fast lane and empty-root page-run
  bulk path. Repeated standalone attempts around row serialization, page-run
  grouping, prebuilt leaf pages, arena/owned buffering, and page image fusion
  have regressed the focused matrix.
- Low-thread concurrent writer rows are dominated by stale-snapshot retry and
  page-lock wait behavior, while file-backed page-run admission and wait-slice
  tuning have already been rejected.
- DML DELETE rows still pay transaction/MVCC publication ceremony around leaf
  local mutations. Cursor shell retention, root-leaf bypasses, multi-leaf
  backlogs, compact-area caching, and stack-entry detaching have all missed the
  focused keep gate.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Verdict |
| --- | ---: | ---: | ---: | ---: | --- |
| Transaction-local DML mutation operator with read-your-writes/savepoint/MVCC publication proof | 4 | 3 | 5 | 2.4 | Next source frontier |
| File-backed INSERT page construction plus MVCC publication batching | 4 | 2 | 5 | 1.6 | Design only until a proof harness exists |
| Parallel WAL-style lane redesign for low-thread rows | 3 | 2 | 5 | 1.2 | Below threshold from current profiles |
| Setup/open-state fixed-cost redesign | 2 | 2 | 5 | 0.8 | Below threshold unless profiles shift |

## Next Source Gate

The only source candidate above the optimization threshold is a
transaction-local DML mutation operator: collect ordered page-local
INSERT/UPDATE/DELETE messages inside the transaction, serve read-your-writes
from that buffer, flush page-local batches through one MVCC publication path,
and prove rollback/savepoint/schema-drift behavior before benchmarking.

Required proof before a full quick promotion:

- Focused unit coverage for read-after-write, rollback, savepoint release,
  savepoint rollback, schema drift, duplicate rowids, missing rowids, and mixed
  update/delete on the same page.
- Focused `UPDATE/DELETEThroughput` A/B win for the 5-row, 50-row, and 500-row
  DELETE rows in one measurement window.
- Full quick primary-score neutrality or improvement in the same window.

## Rejected Immediate Patch

No source patch was attempted in this pass. A one-file or one-branch tweak to
INSERT, concurrent retry timing, or retained DELETE leaf-run internals would
repeat a fenced family from the current ledger.
