# DML Leaf-Run Boundary Audit - 2026-05-08

## Scope

Read-only boundary audit at `c7bac5ec6f7a13feff9df7bad5b7e9bc0bf1d6f1`
after the current DML mutation profile landed. This is not a source patch and
not a keep-gate benchmark; it records the remaining non-fenced optimization
shape so the performance campaign does not repeat rejected DML micro-patches.

Local worktree caveat during this audit:

- `docs/progress/perf-negative-results.md` already had an uncommitted peer
  entry for the retained direct-DML cursor rejection.
- `.rch-retained-dml-target/` existed as an untracked cargo target directory.
- Neither path was edited or staged by this pass.

## Current Evidence

`tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/summary.md`
shows the clean quick frontier at 93 scenarios, faster/comparable/slower
`80 / 4 / 9`, weighted score `0.3347931621`, p90 `1.0460722171`, and p99
`1.4474956173`. The full quick worst row was 100-row DELETE at `1.4475x`
with high FSQLite CV, while focused DML only had a stable 100-row UPDATE loss.
Focused INSERT did not reproduce the large-row full-run gap, and concurrent
writers were comparable or faster.

`tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/summary.md`
separates standard setup cost from isolated mutation cost:

- Standard 100-row compare: update `3.34x`, delete `4.67x`.
- Isolated mixed compare: update `2.67x`, delete `4.83x`.
- Isolated update-only: `681 ns/row` FSQLite vs `283 ns/row` C SQLite,
  ratio `2.40x`.
- Isolated delete-only: `1666 ns/row` FSQLite vs `301 ns/row` C SQLite,
  ratio `5.54x`.
- Isolated top flat samples: `memmove` `14.70%`,
  `BtCursor<SharedTxnPageIo>::table_seek_for_insert` `7.89%`, allocator
  `7.66%`, `BtCursor<SharedTxnPageIo>::delete` `6.14%`, page write `2.79%`,
  cell-pointer decode `2.53%`, page fetch/load around `2.4%`, same-size
  overwrite `1.96%`, and pointer rewrite `1.66%`.

CASS remains a weak source for current turns: `cass status --json` reported an
unhealthy stale lexical index last indexed `2026-05-06T11:06:05.165+00:00`,
semantic search missing/needs consent, and a targeted seven-day query for
`frankensqlite same-leaf DML run operator rejected retained cursor microbatch`
returned zero hits. Current commits, Agent Mail, artifacts, and the negative
ledger are therefore the authoritative evidence.

## Current Code Boundary

INSERT already has the page-builder family that the previous frontier notes
were pointing toward:

- `BtCursor::table_bulk_load_empty_root_sorted_records` builds sorted
  monotonic table records into leaf pages and parent divider pages once.
- `BtCursor::table_bulk_append_depth2_right_edge_sorted_records` appends sorted
  records to the right edge of a depth-2 table when the root has separator
  space.
- `Connection::flush_pending_direct_insert_page_run_with_cursor` already calls
  those builders before replaying fallback appends.

Direct UPDATE/DELETE still runs one public statement execution at a time:

- `execute_prepared_direct_simple_update` constructs a fresh direct cursor,
  seeks the rowid, may decode old payload, serializes the new record, and then
  either calls `table_overwrite_current_payload_same_size_no_overflow` or
  delete+prechecked-insert.
- `execute_prepared_direct_simple_delete` constructs a fresh direct cursor,
  seeks the rowid, optionally decodes for retained count/sum cache, then calls
  `cursor.delete`.

The important boundary is that a real DML leaf-run operator must change the
mutation unit itself. It cannot merely retain the cursor shell or buffer rowids
after doing the normal per-row admission seek.

## Negative-Ledger Fence

Standalone shapes already rejected or fenced for this frontier include:

- Retained full direct-DML cursor shell: current uncommitted ledger entry and
  `tests/artifacts/perf/swiftgate-retained-dml-cursor-20260508T1920Z/summary.md`
  show isolated update regressed from `681 ns/row` to `1464-1481 ns/row`, and
  isolated delete regressed from `1666 ns/row` to `2031-2105 ns/row`.
- Fixed-width REAL page-local payload patch: isolated UPDATE improved once,
  but the same-window focused matrix aggregate/tail regressed.
- Deferred UPDATE/DELETE microbatch carry: isolated UPDATE moved only
  `661 ns -> 651 ns`, isolated DELETE stayed flat, and standard 100-row DML
  worsened.
- Earlier retained direct UPDATE/DELETE cursor shell: focused update/delete
  matrix worsened materially despite using `advance_to`.
- Pending direct DELETE leaf-run buffer via repeated seeks: dirty smoke
  measured `3203 ns/delete` vs clean `1754 ns/delete`; it still paid ordinary
  root-to-leaf admission on every row.

Do not retry those shapes as standalone optimizations.

## Opportunity Contract

Only one DML source shape is still plausibly non-fenced:

- Operator: a prepared direct-DML leaf-run primitive that owns one decoded leaf
  page image and applies multiple UPDATE/DELETE mutations under one cursor/page
  writer borrow.
- Admission requirement: avoid per-row root-to-leaf admission. A candidate must
  either batch rowid admission before mutation or prove current/next-leaf
  membership from retained decoded leaf bounds without calling the ordinary
  seek for every row.
- Mutation requirement: apply multiple same-leaf mutations with one page decode,
  one pointer-vector decode, and one staged page write where the leaf remains
  structurally valid.
- Visibility requirement: reads, secondary DML, savepoints, rollback, commit,
  schema drift, QF maintenance, retained count/sum cache, and MemDatabase mirror
  invalidation must see either the flushed btree state or a complete logical
  overlay. The safer first candidate is to flush on every observation boundary
  and prove those boundaries exhaustively.
- Fallback requirement: duplicate/missing rowids, non-monotone rowids, rowids
  crossing leaves, secondary index/triggers/FK/cache-sensitive shapes,
  overflow, page freeblock fragmentation, leaf-draining DELETE, and savepoints
  must return to the existing per-row direct path.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Decision |
| --- | ---: | ---: | ---: | ---: | --- |
| True same-leaf DML mutation-run operator | 5 | 3 | 5 | 3.0 | Only viable source lever; high proof burden. |
| Retain cursor shell plus `advance_to` | 2 | 1 | 3 | 0.7 | Rejected twice; do not retry. |
| Buffer DELETE rowids but seek per row | 2 | 1 | 4 | 0.5 | Rejected; still pays admission cost. |
| Page-local fixed REAL patch | 2 | 1 | 3 | 0.7 | Rejected by focused matrix. |
| Schema/microbatch carry around DML | 1 | 1 | 2 | 0.5 | Rejected; setup/mutation profile no longer justifies it. |

## Keep Gate

A DML leaf-run candidate is worth implementing only if it passes this order:

1. Focused correctness: direct UPDATE/DELETE behavior, read-after-write,
   rollback, savepoint rollback, commit, missing rows, duplicate buffered rowids,
   schema change invalidation, QF delete/update maintenance, and concurrent mode
   default still on.
2. Isolated kernel smoke:
   `perf-update-delete 100 20000 update compare isolated` and
   `perf-update-delete 100 20000 delete compare isolated` must both improve vs
   a same-window clean baseline.
3. Focused matrix:
   `comprehensive-bench --quick --filter update` must improve average,
   geomean/weighted, and p90/p99, without swapping the worst row to another DML
   case.
4. Full quick matrix:
   `comprehensive-bench --quick` must improve weighted score and not increase
   slower-row count or p99.
5. Code gate: `cargo check --workspace --all-targets`, `cargo clippy
   --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
   `ubs` on staged Rust files.

## Decision

No source patch was attempted. The current code already contains the INSERT
page-builder family, and the DML profile/ledger rule out another local helper
patch. The next useful source pass should either implement the true same-leaf
DML mutation-run operator above or explicitly stand down; a smaller DML
micro-optimization is expected to fail the same gates the campaign has already
recorded.
