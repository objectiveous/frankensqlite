# Full quick current profile - 2026-05-07

Agent: CrimsonGorge

## Scope

- Profiled current `main` after the latest insert/record/cursor perf commits.
- Focused on the remaining C SQLite faster rows in the full quick matrix:
  small explicit UPDATE/DELETE, tiny 100-row INSERT families, and concurrent
  writer rows.
- Tried and rejected a direct UPDATE/DELETE route-gate hoist.
- Tried and rejected a prepared direct INSERT append-hint active bit before
  benchmarking because it failed the focused correctness gate.

## Current full quick signal

Report: `full-dirty-cursor-report.json`

- Scenarios: 93
- Franken faster: 78
- Comparable: 5
- C SQLite faster: 10
- Average ratio: 0.46677516073792624
- Geomean ratio: 0.26937297239408864
- Median ratio: 0.2943379172315837
- P90 ratio: 1.0565599644525723
- P99 ratio: 1.5209170437405732
- Weighted score: 0.3540566922651954

## Rejected candidates

### Direct UPDATE/DELETE autocommit probe gate hoist

Artifacts:

- `update-baseline-rerun-report.json`
- `update-dml-gate-hoist-report.json`
- `stdout/update-baseline-rerun.*`
- `stdout/update-dml-gate-hoist.*`
- `stdout/build-dml-gate-hoist.*`

Focused update/delete geomean worsened from `1.0406236970466178` to
`1.2087296154254785`; average worsened from `1.0661450609718544` to
`1.2287248777917592`. The candidate was restored manually.

### Prepared direct INSERT append-hint active bit

No benchmark artifact was produced. The candidate failed
`cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture` before
measurement. The source was restored manually and the stale current-tree core
test expectation was updated to match the B-tree staged-page mutation invariant:
retained autocommit must keep table-local append state parked, but staged-page
mutation may drop duplicate retained page bytes.

## Notes

- `perf-update-current.data` is a bad first perf capture with only a few samples
  because the `perf record` option order was wrong.
- `perf-update-current-v2.data` is the valid focused UPDATE/DELETE perf sample.
- `docs/progress/perf-negative-results.md` records the durable rejection notes.
