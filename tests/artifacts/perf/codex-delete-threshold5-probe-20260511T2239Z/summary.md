# Compact DELETE Threshold-5 Probe

- Source candidate: lowered `COMPACT_DELETE_SINGLE_PASS_MIN` from `6` to `5`.
- Benchmark command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update`.
- Result: rejected. The candidate did not improve all DELETE rows in the same window.

| Scenario | FSQLite median | C SQLite median | Ratio |
|---|---:|---:|---:|
| 100 rows / delete 5 rows | 0.007033 ms | 0.002365 ms | 2.9738x |
| 1000 rows / delete 50 rows | 0.029295 ms | 0.016261 ms | 1.8015x |
| 10000 rows / delete 500 rows | 0.258945 ms | 0.162354 ms | 1.5949x |
