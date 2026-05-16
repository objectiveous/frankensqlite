# Dense B-tree Rowid DELETE Candidate Fixed-Run Rejection

- Date: 2026-05-16
- Candidate area: `crates/fsqlite-core/src/connection.rs`
- Target command shape: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update-delete`
- Raw remote artifacts: not retained locally. This file is the durable
  rejection summary for the session-captured fixed-candidate output.
- Baseline reference:
  `tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/summary.md`
- Correctness-failing first run:
  `tests/artifacts/perf/codex-dense-btree-rowid-delete-candidate-20260516T095949Z/summary.md`

## Result

Rejected and unwound uncommitted.

The fixed candidate restored the cursor when the dense oracle skipped a table,
so the correctness proof passed. It then admitted the target DELETE path, but
made the important row worse:

- `10000 rows / delete 500 rows`: FrankenSQLite regressed from the current
  profile range around `421-435 us` to about `765.7 us`.
- `commit_us=586.1`
- `direct_flush_ns=502158`
- `delete_leaf_start=0/0`
- `delete_leaf_active=0/0`
- `delete_leaf_flush=0/0`
- `delete_leaf_materialize=64`
- `delete_leaf_search=560`

## Root Cause

The new path skipped the retained leaf-run admission counters, but moved the
cost into commit-time materialization and per-row search during flush. That is
the wrong shape for the DML gap: it must become O(number of touched leaves),
not a deferred replay of row-by-row physical work.

## Retry Boundary

Do not retry standalone dense B-tree proof plus deferred rowid set flush.
Reconsider only if the flush becomes a true leaf/range-batched mutation
operator, avoids per-row `advance_to`/materialize/search churn, and preserves
cursor position before any fallback path.
