# Transaction-Local DML Mutation Boundary

- Date: 2026-05-12
- Git: `05c28b8b7bc5d7f3359ad8cfe0e038fd6a4f51f7`
- Source state: clean current frontier before this artifact
- Target: remaining `UPDATE/DELETEThroughput` rows after the page-1 skip,
  per-mutation UPDATE/DELETE, direct-flush, and low-thread concurrent families
  were fenced by the negative-results ledger.

## Current Red Rows

From `tests/artifacts/perf/codex-current-frontier-fullquick-20260512T0810Z/full.json`:

| Row | F/C ratio |
| --- | ---: |
| `100 rows / delete 5 rows` | `2.970x` |
| `1000 rows / delete 50 rows` | `1.847x` |
| `10000 rows / delete 500 rows` | `1.626x` |
| `100 rows / update 10 rows` | `1.409x` |

From `tests/artifacts/perf/codex-current-clean-dml-profile-20260512T0945Z/current-update.json`:

| Row | C SQLite | FrankenSQLite | F/C ratio |
| --- | ---: | ---: | ---: |
| `100 rows / update 10 rows` | `0.004328 ms` | `0.006282 ms` | `1.451x` |
| `100 rows / delete 5 rows` | `0.002314 ms` | `0.007133 ms` | `3.083x` |
| `1000 rows / update 100 rows` | `0.037410 ms` | `0.028193 ms` | `0.754x` |
| `1000 rows / delete 50 rows` | `0.015990 ms` | `0.029405 ms` | `1.839x` |
| `10000 rows / update 1000 rows` | `0.378869 ms` | `0.247253 ms` | `0.653x` |
| `10000 rows / delete 500 rows` | `0.160731 ms` | `0.258264 ms` | `1.607x` |

The profile confirms direct DML dispatch is already active: DELETE rows report
`slow=0`; the 500-row DELETE reports `delete_leaf_active=433/496`,
`delete_leaf_flush=64/64`, `delete_leaf_materialize=64/37672`,
`delete_leaf_write=64/7431`, and `commit_us=38.9`. The 100-row UPDATE reports
`direct_update=10`, `begin_ns=2425`, `execute_body_ns=7724`,
`prepared_lookup_ns=2143`, `commit_roundtrip_ns=2014`, and `commit_us=6.8`.

## Closed Standalone Levers

The current ledger explicitly fences these as standalone retries:

- Exact transaction-control SQL bypass.
- Prepared-cache, background-status, prepared-lookup, and wrapper trimming.
- Direct UPDATE payload/assignment/cursor tweaks.
- Retained DELETE leaf-run admission, search, materialization, direct writer,
  direct flush, cursorless flush, scratch reset, and rowid-buffer variants.
- Normal private-memory page-1 commit skipping.
- Small INSERT fixed-cost tails and low-thread concurrent retry/wait shaping.

This rules out another narrow patch that only removes a cursor wrapper,
changes a leaf-run threshold, records a cell-log delete without read
integration, or trims a fixed-cost statement envelope.

## Existing Support

The MVCC crate already has the lower-level pieces for a transaction-local
logical DML representation:

- `crates/fsqlite-mvcc/src/cell_visibility.rs` has
  `CellVisibilityLog::{record_insert,record_update,record_delete}`,
  `resolve_for_txn`, transaction bulk commit, rollback, savepoint rollback
  support, and page-level delta counts.
- `crates/fsqlite-mvcc/src/lifecycle.rs` exposes `TransactionManager::cell_log`,
  commits cell deltas with the transaction commit sequence, rolls them back on
  abort, and records savepoint delta lengths.
- `crates/fsqlite-mvcc/src/materialize.rs` can apply committed cell deltas to a
  base B-tree leaf page and now reports live materialized cell counts.

## Missing Boundary

The pieces above are not yet a safe source lever for the current benchmark
rows because the core B-tree execution path still observes physical page
images, not a transaction-local logical read view:

- `TransactionManager::read_page` checks `txn.write_set_data` first and then
  resolves the visible full page version. It does not materialize committed
  cell deltas for normal B-tree traversal and does not overlay this
  transaction's uncommitted cell deltas.
- `Connection` direct UPDATE/DELETE paths maintain pending physical leaf-run
  buffers. They do not publish logical rowid tombstones or updates into
  `CellVisibilityLog`.
- Read-your-writes for `SELECT`, subsequent DML, savepoints, rollback, and
  commit publication currently depends on flushing those physical pending
  write runs before read boundaries.

A commit-side-only `cell_log.record_delete()` hook would therefore be wrong:
it could report affected rows and update commit metadata while the next B-tree
read still sees the old physical row image.

## Source-Change Contract

The next credible code lever is a transaction-local rowid DML operator, not a
single micro-tweak. It needs this minimum contract before benchmarking:

1. Represent rowid DML deltas by `(root_page, rowid)` with insert/update/delete
   state transitions and coalescing inside one transaction.
2. Provide a B-tree read adapter that overlays committed visible cell deltas and
   this transaction's uncommitted deltas for point reads and scans.
3. Flush or materialize at explicit fallback boundaries: structural B-tree
   changes, unsupported predicates, index maintenance, virtual tables,
   savepoint modes not covered by delta rollback, and any path that requires
   exact physical page bytes.
4. Preserve direct-DML correctness tests for pending update/delete leaf runs,
   savepoint rollback, read-before-commit, rollback, and autocommit.
5. Keep the same-window gates: focused `--quick --filter update` must improve
   FSQLite absolute medians for the DELETE rows and preserve the green larger
   UPDATE rows; full quick must be primary-score neutral or better.

## Decision

No source patch was attempted from this pass. The safe next action is to design
and implement the broader transaction-local logical mutation operator behind
the read-view boundary above. Another standalone physical leaf-run, page-1,
direct-flush, or fixed-envelope patch would repeat fenced negative work.
