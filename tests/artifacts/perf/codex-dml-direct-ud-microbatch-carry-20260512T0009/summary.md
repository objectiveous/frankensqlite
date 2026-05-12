# Direct UPDATE/DELETE Microbatch Carry Probe

- Date: 2026-05-12.
- Source baseline: `fcccad72ba4579bf39aaa54cfecef9aa8ad98e13` plus a local
  candidate patch in `crates/fsqlite-core/src/connection.rs`.
- Candidate: allow the prepared-DML microbatch carry to bypass the conservative
  `may_observe_change_tracking` flag for direct-simple UPDATE/DELETE, and use
  it in the programless direct UPDATE/DELETE path.
- Build: local release-perf,
  `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-next-target`.
- Command:
  `FSQLITE_BENCH_PROFILE_DML=1 /tmp/frankensqlite-codex-next-target/release-perf/comprehensive-bench --quick --filter dml --no-html --json-out <run>.json`.
- Artifacts:
  - `run1.json`
  - `run2.json`

## Baseline Reference

`tests/artifacts/perf/codex-dml-frontier-repeat-20260511Tnext/summary.md`
reported:

| Scenario | Baseline FSQLite median | Baseline C SQLite median | Baseline F/C |
|---|---:|---:|---:|
| 100 rows / update 10 rows | 0.006352 ms | 0.004218 ms | 1.50593x |
| 100 rows / delete 5 rows | 0.007113 ms | 0.002294 ms | 3.10070x |
| 1000 rows / update 100 rows | 0.028283 ms | 0.036328 ms | 0.77855x |
| 1000 rows / delete 50 rows | 0.028934 ms | 0.016200 ms | 1.78605x |
| 10000 rows / update 1000 rows | 0.246291 ms | 0.353713 ms | 0.69630x |
| 10000 rows / delete 500 rows | 0.260207 ms | 0.162966 ms | 1.59670x |

## Candidate Runs

| Scenario | Run 1 F/C | Run 2 F/C | Decision |
|---|---:|---:|---|
| 100 rows / update 10 rows | 1.37231x | 1.30518x | better |
| 100 rows / delete 5 rows | 2.92367x | 2.66654x | better |
| 1000 rows / update 100 rows | 0.79384x | 0.80721x | worse |
| 1000 rows / delete 50 rows | 1.78510x | 2.30021x | mixed/regressed |
| 10000 rows / update 1000 rows | 0.70238x | 0.71820x | worse |
| 10000 rows / delete 500 rows | 1.65012x | 1.76931x | regressed |

The patch did reduce the intended fixed ceremony: the DML profile showed
`schema_refreshes=1` for direct UPDATE/DELETE batches. That did not translate
into a stable section win; the larger DELETE rows regressed in one or both exact
DML runs.

## Decision

Rejected and manually unwound. Do not retry direct-simple UPDATE/DELETE
microbatch carry as a standalone patch. Reconsider only as part of the broader
transaction-local DML mutation operator if a same-window A/B improves all
focused DML medians and the full quick matrix stays neutral or better.
