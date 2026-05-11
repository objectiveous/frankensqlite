# Current-Head DML Hotpath Profile - 2026-05-11

Purpose: same-HEAD focused `UPDATE/DELETEThroughput` baseline before retrying
the exact transaction-control execute fast path.

Source context from benchmark stdout:

- Git: `main @ 56b73f08a36e8161fccf434d79d56ea006dcd6a7`.
- Build: `release-perf` with opt-level 3 and LTO.
- Command shape:
  `env FSQLITE_BENCH_PROFILE_DML=1 .../comprehensive-bench --quick --filter update-delete --no-html --json-out tests/artifacts/perf/codex-current-head-dml-hotpath-20260511T0652Z/update-delete.json`.

Focused DML rows:

| Scenario | FSQLite median ms | C SQLite median ms | F/C ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.006582 | 0.004338 | 1.5172890733 |
| 100 rows / delete 5 rows | 0.007835 | 0.002235 | 3.5055928412 |
| 1000 rows / update 100 rows | 0.030597 | 0.036187 | 0.8455246359 |
| 1000 rows / delete 50 rows | 0.032220 | 0.015869 | 2.0303736845 |
| 10000 rows / update 1000 rows | 0.269325 | 0.368861 | 0.7301530929 |
| 10000 rows / delete 500 rows | 0.291786 | 0.159428 | 1.8302054846 |

Profile note: explicit `BEGIN` / `COMMIT` ceremony still passed through the
normal execute-body path in this baseline (`parser_multi_calls=2`,
`execute_body_ns` nonzero, and `direct_flush_calls=2` in the DML profile
lines).
