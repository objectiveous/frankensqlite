# Dense B-tree Rowid DELETE Candidate Correctness Failure

- Date: 2026-05-16
- Candidate area: `crates/fsqlite-core/src/connection.rs`
- Target command shape: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update-delete`
- Raw remote artifacts: not retained locally. This file is the durable
  correctness-failure summary for the session-captured candidate output.

## Result

Rejected and unwound uncommitted.

The candidate scanned the table B-tree on the first eligible private-memory
prepared rowid DELETE to prove a dense rowid interval, buffered exact
transaction-local deleted rowids, and materialized physical B-tree deletes at
the normal read/commit boundary.

The first run failed during `fs_delete_100` teardown with:

```text
PRIMARY KEY constraint failed
```

## Root Cause

When the dense oracle skipped a small table, it left the cursor parked at the
last row. The fallback deletion path then operated from the wrong physical
cursor position. A local fix restored the cursor before fallback and passed the
focused proof tests, but that fixed candidate still regressed the benchmark.
See:

`tests/artifacts/perf/codex-dense-btree-rowid-delete-candidate-fixed-20260516T102402Z/summary.md`
