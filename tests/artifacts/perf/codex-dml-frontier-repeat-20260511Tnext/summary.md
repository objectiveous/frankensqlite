# DML Frontier Repeat After Retained Delete-Run Commit

- Date: 2026-05-11.
- Source: `2d47211078847260f63adc496eb9e44e262a850c`.
- Build: local release-perf, `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-next-local-target`.
- Artifact: `tests/artifacts/perf/codex-dml-frontier-repeat-20260511Tnext/`.
- Command:
  `env FSQLITE_BENCH_PROFILE_DML=1 /tmp/frankensqlite-codex-next-local-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/codex-dml-frontier-repeat-20260511Tnext/update-delete.json --no-html`.

## Result

| Scenario | FSQLite median | C SQLite median | Ratio |
|---|---:|---:|---:|
| 100 rows / update 10 rows | 0.006352 ms | 0.004218 ms | 1.50593x |
| 100 rows / delete 5 rows | 0.007113 ms | 0.002294 ms | 3.10070x |
| 1000 rows / update 100 rows | 0.028283 ms | 0.036328 ms | 0.77855x |
| 1000 rows / delete 50 rows | 0.028934 ms | 0.016200 ms | 1.78605x |
| 10000 rows / update 1000 rows | 0.246291 ms | 0.353713 ms | 0.69630x |
| 10000 rows / delete 500 rows | 0.260207 ms | 0.162966 ms | 1.59670x |

## Attribution

All DELETE rows stayed on the prepared direct path (`slow=0`).

- `100 rows / delete 5 rows`: one same-leaf run, one dirty flush,
  `delete_leaf_flush_ns=1673`, `delete_leaf_materialize=1/1152`.
- `1000 rows / delete 50 rows`: six dirty leaf-run flushes,
  `delete_leaf_active=44/49`, `delete_leaf_miss_out_of_leaf=5`,
  `delete_leaf_flush_ns=8816`.
- `10000 rows / delete 500 rows`: 64 dirty leaf-run flushes,
  `delete_leaf_active=433/496`, `delete_leaf_miss=63`,
  `delete_leaf_flush_ns=114393`, `delete_leaf_materialize=64/97923`,
  `delete_leaf_write=64/8977`.

## Decision

No source patch from this repeat. The profile does not invalidate the existing
negative ledger: standalone retained delete-run tweaks, direct-flush wrappers,
parent-separator admission, tombstone-only overlays, dense-rowid queues,
microbatch carry, and exact transaction-control bypasses remain fenced.

The next admissible implementation is still the broader transaction-local DML
mutation operator for `bd-db300.11.1`, with read-your-writes, rollback,
savepoint, duplicate/missing rowid, schema drift, cache/QF invalidation, and
MVCC publication proof before any focused/full-quick keep gate.
