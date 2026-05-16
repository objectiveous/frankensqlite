# Dense MemDatabase Rowid DELETE Candidate Rejection

- Date: 2026-05-16
- Candidate area: `crates/fsqlite-core/src/connection.rs`
- Target command shape: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update-delete`
- Raw remote artifacts: not retained locally. This file is the durable
  rejection summary for the session-captured candidate output.
- Baseline reference:
  `tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/summary.md`

## Result

Rejected and unwound uncommitted.

The candidate buffered exact transaction-local DELETE rowids for dense
private-memory tables when a clean `MemDatabase` row mirror could prove the
affected count, then materialized physical B-tree deletes at the normal
read/commit boundary.

The apparent 10000-row DELETE movement was not a valid keep signal because the
candidate did not admit the benchmark workload. The candidate profile still
reported the retained leaf-run path:

- `delete_leaf_start=64/67`
- `delete_leaf_active=433/496`
- `delete_leaf_miss=63`
- `delete_leaf_flush=64/64`

## Root Cause

The proof tests used the default time-travel-capturing mode. The benchmark uses
`PRAGMA fsqlite_capture_time_travel_snapshots=false`, leaving the `MemDatabase`
row mirror lazy after setup commits. That makes the clean-memdb dense-rowid
oracle unavailable for the measured workload.

## Retry Boundary

Do not retry a standalone dense-rowid DELETE buffer gated on
`memdb_rows_loaded && memdb_storage_count_shortcuts_safe`. Reconsider only as
part of the broader transaction-local DML mutation operator with an exact
affected-row oracle that works in snapshot-free/lazy-MemDatabase benchmark mode.
