# DML Mutation Operator Source Feasibility Probe

Date: 2026-05-13

Base commit:
`e644bd64eefea85d67e0eb9a813eacee3b2790de`
(`fix(mvcc): lock cell-delta-only commit pages`)

Published frontier artifact:
`tests/artifacts/perf/codex-e644bd64-frontier-refresh-20260513T0248Z/`

## Question

Can the remaining `UPDATE/DELETEThroughput` DELETE deficit be fixed by a narrow
source patch after the retained same-leaf and monotone DELETE work, or does it
need the broader transaction-local DML mutation operator from
`bd-db300.11.1`?

## Current Measured Frontier

The `e644bd64` full quick matrix reported `93` scenarios with FrankenSQLite
faster / comparable / C-SQLite-faster at `78 / 6 / 9`, average F/C
`0.4964158116`, geomean F/C `0.2752616803`, p99 F/C `3.0527225583`, and primary
weighted score `0.3710116820`.

Remaining DELETE rows are still the largest C-SQLite-faster tail:

| Scenario | C median ms | F median ms | F/C |
|---|---:|---:|---:|
| `100 rows / delete 5 rows` | `0.002305` | `0.007113` | `3.0859x` |
| `1000 rows / delete 50 rows` | `0.015930` | `0.028924` | `1.8157x` |
| `10000 rows / delete 500 rows` | `0.158206` | `0.276939` | `1.7505x` |

Representative `fs_delete_10000` counters from the focused profile:

- `direct_delete=500`, `fast=500`, `slow=0`
- `delete_leaf_active=433/496`, `delete_leaf_miss=63`
- `delete_leaf_flush=64/64`
- `delete_leaf_materialize=64/39933ns`
- `delete_leaf_write=64/7453ns`
- `delete_leaf_search=560/39508ns`
- `execute_body_ns=41287`
- `commit_roundtrip_ns=20789`

## Live Direct DELETE Path

The current direct prepared DELETE path is already on the retained page-image
fast path:

- `PendingDirectDeleteLeafRun` stores one current leaf image plus
  `leaf_max_rowid` metadata in `crates/fsqlite-core/src/connection.rs:7584`.
- Connection state keeps one active run plus staged monotone prior runs at
  `crates/fsqlite-core/src/connection.rs:7882`.
- `execute_prepared_direct_simple_delete` flushes pending update/insert runs,
  tries the active retained DELETE run first, and otherwise creates a B-tree
  cursor over `SharedTxnPageIo` at `crates/fsqlite-core/src/connection.rs:18290`.
- Pending DELETE runs flush through a cursor and restore all buffers on error at
  `crates/fsqlite-core/src/connection.rs:31925`.
- The active run attempts `delete_rowid_with_reason`, stages only monotone
  cross-leaf rowids, and otherwise flushes at
  `crates/fsqlite-core/src/connection.rs:32220`.
- `TableLeafDeleteRun` owns an opaque table-leaf image and rejects deletes that
  would need parent separator repair, overflow cleanup, or rebalance in
  `crates/fsqlite-btree/src/cursor.rs:1342`.
- The current search path already has first/last probes, dense-rowid exact slot
  detection, interpolation probes, and binary-search fallback around
  `crates/fsqlite-btree/src/cursor.rs:3624`.

This leaves little room for another safe micro-patch. The obvious narrow ideas
map to already rejected ledger entries: retained-run search hints,
duplicate-check changes, compactness/materialization tweaks, direct-flush
wrappers, publication shortcuts, and tombstone-only overlays.

## Cell-Delta Primitive Exists, But Not On This Path

The MVCC crate has the logical cell-delta pieces needed by a real operator:

- `MvccManager::read_page_with_cell_deltas` reads a page and materializes
  visible committed plus own uncommitted deltas at
  `crates/fsqlite-mvcc/src/lifecycle.rs:807`.
- `commit` treats cell deltas as writes even without a full page image at
  `crates/fsqlite-mvcc/src/lifecycle.rs:1122`.
- `abort` rolls back uncommitted cell deltas at
  `crates/fsqlite-mvcc/src/lifecycle.rs:1151`.
- Savepoints snapshot and prune `cell_delta_len` at
  `crates/fsqlite-mvcc/src/lifecycle.rs:1171` and
  `crates/fsqlite-mvcc/src/lifecycle.rs:1199`.
- Cell-delta-only commit exclusion locks affected pages and records write
  witnesses at `crates/fsqlite-mvcc/src/lifecycle.rs:1424`.
- Tests cover savepoint rollback and read-view materialization, including
  `test_rollback_to_savepoint_prunes_cell_deltas` at
  `crates/fsqlite-mvcc/src/lifecycle.rs:3513`.

The live core/VDBE/pager path does not currently expose that primitive:

- `SharedTxnPageIo` wraps a pager `TransactionKind` and optional concurrent
  context, not an `MvccManager` transaction, at
  `crates/fsqlite-vdbe/src/engine.rs:1722`.
- `PageReader::read_page_data` and `read_btree_page_data` record page witnesses
  but return `self.txn.borrow().get_page(...)` directly at
  `crates/fsqlite-vdbe/src/engine.rs:2740` and
  `crates/fsqlite-vdbe/src/engine.rs:2769`.
- `TransactionHandle` is page-oriented: `get_page`, `write_page`,
  `write_page_data`, `commit`, savepoint, and rollback, with no logical
  row/cell mutation API at `crates/fsqlite-pager/src/traits.rs:616`.
- A source search for `cell_log`, `CellVisibilityLog`, and
  `read_page_with_cell_deltas` under `crates/fsqlite-core`,
  `crates/fsqlite-vdbe`, and `crates/fsqlite-pager` found no live integration
  point. The hits are QF bookkeeping or MVCC-lifecycle-local tests and helpers.

## Decision

No source patch was attempted from this probe.

A narrow retained-run edit would either repeat a measured rejection or optimize
the physical page-image path that is already carrying the DELETE rows. The next
worthwhile source change is the broader transaction-local DML mutation operator
that bridges logical rowid/key messages into the live `SharedTxnPageIo` /
`TransactionHandle` path and gives B-tree reads a correct delta-aware read view.

## Required First Slice

Before touching performance-sensitive DELETE mechanics again, the first
implementation slice needs to define the integration boundary:

- transaction-owned logical DELETE/UPDATE/INSERT message storage keyed by
  stable rowid/key identity, not physical cell index;
- read-your-writes overlay for B-tree point lookups, scans, row counts, QF, and
  retained count-cache invalidation;
- rollback and savepoint ownership that prunes logical messages with the same
  semantics as full-page write sets;
- conflict publication through the same MVCC page/cell witness surface used by
  concurrent writers;
- focused rusqlite oracles for affected row counts, duplicate/missing rowids,
  rollback, savepoints, schema drift, and concurrent mode defaults;
- keep gate: two focused `--quick --filter update` runs improving all three
  DELETE medians without UPDATE regression, then two full quick matrix runs
  with primary-score neutrality or better.

## Canonical-Branch Safety Note

This probe was performed from the clean canonical worktree at `origin/main`.
The dirty `/data/projects/frankensqlite` checkout remains untouched. Its local
`main` branch is an ancestor of `origin/main` and has no commits ahead, while
its working tree diff is deletion-heavy; none of that stale worktree state was
imported into the canonical branch.
