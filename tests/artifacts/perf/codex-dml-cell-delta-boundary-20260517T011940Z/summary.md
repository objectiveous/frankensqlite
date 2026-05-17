# DML Cell-Delta Boundary Refresh

- Date: 2026-05-17 01:19:40 UTC.
- Source: `main @ a557b20b243117d4fc971e6e670e79ace22ec6c4`.
- Scope: profile-first selection for the next DELETE/DML performance lever.
- Source patch: none.

## Inputs Refreshed

- Recent commits show the live branch just past the Windows VFS sidecar cleanup stack; the current hot Linux benchmark path is still represented by the May 16 DML, INSERT, concurrent, and full-quick perf artifacts.
- `br ready --json` is still blocked by a malformed Beads database: page 3552 has invalid B-tree page type `0x00`.
- `bv --robot-triage` returned no recommendations (`null`, `[]`).
- CASS lexical search is stale and semantic search is unavailable; focused searches for the current DML/cell-delta terms returned zero hits.
- The graveyard corpus maps FrankenSQLite's current shape to B-epsilon message buffers, latch-free MVCC, parallel WAL, S3-FIFO buffer pool work, and vectorized execution. For the current measured DELETE frontier, B-epsilon-style row/key mutation messages are the only candidate that directly matches the observed retained-leaf ceremony.

## Current Measured Frontier

Latest focused DML artifact:
`tests/artifacts/perf/codex-current-dml-profile-staged-miss-20260516T193530Z/summary.md`.

| Scenario | C SQLite | FrankenSQLite | Result |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 2.6 us | 6.6 us | 2.53x slower |
| 1000 rows / delete 50 rows | 18.1 us | 32.3 us | 1.78x slower |
| 10000 rows / delete 500 rows | 160.2 us | 335.3 us | 2.09x slower |

Representative 10K/500 profile:

| Counter | Value |
| --- | ---: |
| direct DELETE fast path | `direct_delete=500`, `slow=0`, `vdbe_opcodes=0` |
| retained leaf hits | `delete_leaf_start=64/67`, `delete_leaf_active=433/496` |
| retained leaf misses | `delete_leaf_miss=63`, `out_of_leaf=60`, `last_cell=3`, `staged=0` |
| retained leaf flush | `delete_leaf_flush=64/64`, `delete_leaf_flush_ns=88203` |
| materialize/search/write | `materialize=64/68324`, `search=560/67225`, `write=64/11528` |
| commit envelope | `direct_flush_ns=13450`, `commit_roundtrip_ns=42123` |

Latest current full-quick refresh:
`tests/artifacts/perf/codex-current-fullquick-refresh-20260516T204549Z/run.log`.

- 93 scenarios.
- FrankenSQLite faster / comparable / C-SQLite-faster: `76 / 5 / 12`.
- Average F/C: `0.53x`.
- Remaining red rows include DELETE, large 10-column INSERT, and low-thread same-table concurrent writers.

## Source Boundary

The live direct DELETE path is already the retained same-leaf path:

- `crates/fsqlite-core/src/connection.rs::execute_prepared_direct_simple_delete` first flushes conflicting update/insert runs, then tries an active `TableLeafDeleteRun`.
- `crates/fsqlite-core/src/connection.rs::try_execute_prepared_direct_simple_delete_active_leaf_run` stages monotone cross-leaf runs when rowids advance beyond the current leaf.
- `crates/fsqlite-core/src/connection.rs::flush_pending_direct_delete_leaf_run` reuses one B-tree cursor for same-shape staged runs and materializes each retained leaf once.
- `crates/fsqlite-btree/src/cursor.rs::TableLeafDeleteRun` stores a compact leaf image plus logical deleted cell indices, then materializes a full page image at the boundary.

This means the remaining DELETE gap is not a missing obvious fast path. It is the accumulated cost of per-row retained-leaf ceremony plus page-image publication.

## Negative Gates

The current ledger already fences the obvious smaller ideas:

- No standalone retained DELETE search, duplicate-check, compactness, materializer, parent-separator, last-cell, freeblock, or direct-flush patch.
- No standalone dense-rowid MemDatabase or B-tree-proven rowid buffer.
- No tombstone-only overlay without exact read, rollback, savepoint, and MVCC publication semantics.
- No staged-run-specific patch, because `delete_leaf_miss_staged=0` in the current profile.
- No per-connection memory-concurrent synced-root cache; prior attempt was invalid across transaction lifetimes.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Decision |
| --- | ---: | ---: | ---: | ---: | --- |
| Transaction-local row/key DML mutation operator | 5 | 4 | 4 | 5.0 | Keep as next implementation boundary |
| Fused large-row INSERT row/body/page construction | 4 | 3 | 4 | 3.0 | Secondary after DELETE unless a fresh INSERT profile isolates a safer helper |
| Low-thread concurrent publication batching | 4 | 3 | 4 | 3.0 | Secondary; must prove SSI/FCW and same-table rows |
| S3-FIFO buffer pool replacement | 3 | 3 | 3 | 3.0 | Useful but not directly matched to the current top red DELETE row |
| Retained leaf-run micro-tweak | 1 | 2 | 1 | 2.0 | Rejected by current negative ledger despite low effort |

## Recommendation Contract

### Transaction-Local Row/Key DML Mutation Operator

- Primitive: B-epsilon-style message buffer in logical row/key space.
- Runtime artifact: transaction-local mutation buffer keyed by root/table and rowid/key, not by physical cell index.
- Merge boundary: proven first read of affected data or commit, with explicit rollback/savepoint ownership.
- Read visibility: read-your-writes for point reads and scans before commit.
- Publication: same MVCC/page-cell conflict surface as the physical DELETE path, with no weakening of concurrent mode defaults.
- Correctness proof:
  - affected row counts match C SQLite for existing, missing, duplicate, and repeated rowids;
  - rollback and nested savepoints restore pre-buffer visibility;
  - QF/count-cache/MemDatabase mirrors are invalidated or updated exactly;
  - schema drift rejects stale buffered mutations safely;
  - concurrent conflict witnesses are no weaker than physical page writes.
- Performance keep gate:
  - two focused `comprehensive-bench --quick --filter update-delete` runs improve the 5-, 50-, and 500-row DELETE medians without regressing UPDATE;
  - two full quick runs are primary-score neutral or better with no new critical red rows.
- Rollback: revert the single optimization commit; do not ship a compatibility shim or an off-by-default implementation.

## Decision

No source optimization was attempted from this refresh. The next source change should start at the transaction-local mutation boundary above. Any smaller retained-leaf or cursor-materializer patch would repeat a measured rejection unless a newer profile changes the hotspot table first.
