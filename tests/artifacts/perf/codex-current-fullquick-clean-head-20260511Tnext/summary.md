# Clean current-HEAD full quick benchmark

- Date: 2026-05-11 23:18 UTC
- Commit: `de6fc49c67f83eb240129742918403f852ef9c54`
- Command: `env FSQLITE_BENCH_PROFILE_DML=0 FSQLITE_BENCH_PROFILE_INSERT=0 /tmp/frankensqlite-codex-current-head-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/codex-current-fullquick-clean-head-20260511Tnext/full.json --no-html`
- Note: the benchmark binary was rebuilt after `de6fc49c`; the run does not contain the earlier stale-binary warning. The worktree still reported `Git dirty: yes` because this shared checkout has untracked/ignored artifact directories, while `git status --short` showed no tracked source changes.

## Summary

- Total scenarios: 93
- FrankenSQLite faster: 79
- Comparable: 3
- C SQLite faster: 11
- Primary weighted score: `0.3690581003343589`
- Geomean ratio: `0.2728949849561178`
- Median ratio: `0.29813868134748983`
- P90 ratio: `1.086745339258617`
- P99 ratio: `3.134502923976608`

## Rows still above parity

| Section | Scenario | FSQLite median ms | C SQLite median ms | F/C ratio |
| --- | --- | ---: | ---: | ---: |
| UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.007504 | 0.002394 | 3.134503 |
| UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 0.030718 | 0.016530 | 1.858318 |
| UPDATE/DELETEThroughput | 10000 rows / delete 500 rows | 0.262502 | 0.164488 | 1.595873 |
| UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.006302 | 0.004308 | 1.462860 |
| Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 13.650575 | 11.074500 | 1.232613 |
| INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.075892 | 0.064531 | 1.176055 |
| INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.084609 | 0.073438 | 1.152115 |
| INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.082614 | 0.072436 | 1.140510 |
| INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.083186 | 0.073918 | 1.125382 |
| INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.160421 | 0.147616 | 1.086745 |
| INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.106068 | 0.100058 | 1.060065 |
| Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 4 writers x 1000 rows | 18.990044 | 18.583773 | 1.021862 |

## Interpretation

This recertifies the current frontier after the retained DELETE leaf-run work:
the dominant remaining gap is still explicit-transaction direct DELETE,
especially the 5-row fixed-cost case and the 50/500-row physical mutation
tail. The full quick matrix does not point to another standalone
`TableLeafDeleteRun` threshold/admission tweak; that family is already fenced
in `docs/progress/perf-negative-results.md`.
