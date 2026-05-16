# Direct DML Microbatch Carry Candidate Rejection

- Date: 2026-05-16
- Candidate area: `crates/fsqlite-core/src/connection.rs`
- Target command shape: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update-delete`
- Raw remote artifacts: not retained locally. This file is the durable
  rejection summary for the session-captured candidate output.
- Baseline reference:
  `tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/summary.md`

## Result

Rejected and unwound uncommitted.

The candidate let explicit-transaction programless direct-simple prepared
UPDATE/DELETE statements reuse statement microbatch schema-proof carry even
though UPDATE/DELETE conservatively report
`may_observe_change_tracking = true`.

It did not materially move the target matrix:

- `100 rows / update 10 rows`: FrankenSQLite stayed at about `8.7 us`.
- `100 rows / delete 5 rows`: FrankenSQLite moved only from about `9.8 us`
  to about `9.5 us`, with high candidate variance.
- `1000 rows / update 100 rows`: FrankenSQLite stayed effectively flat,
  about `44.0 us` to `43.9 us`.
- DELETE rows remained C-SQLite-faster.

## Root Cause

The profile already showed only one schema refresh per measured batch, so the
candidate removed little or no hot-loop work. Fresh review also classified it
as a repeat of the rejected 2026-05-07 and 2026-05-12 microbatch-carry family.

## Retry Boundary

Do not retry standalone direct-simple UPDATE/DELETE schema-proof carry for the
current update-delete matrix. Reconsider only if a new benchmark shape proves
repeated schema validation inside the mutation loop and the full quick matrix
moves.
