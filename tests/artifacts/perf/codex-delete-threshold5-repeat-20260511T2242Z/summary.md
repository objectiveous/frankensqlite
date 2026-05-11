# Compact DELETE Threshold-5 Repeat

- Source candidate: lowered `COMPACT_DELETE_SINGLE_PASS_MIN` from `6` to `5`.
- Benchmark command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update`.
- Result: rejected. The repeat regressed the 500-row DELETE median versus the threshold-6 baseline.

| Scenario | FSQLite median | C SQLite median | Ratio |
|---|---:|---:|---:|
| 100 rows / delete 5 rows | 0.007043 ms | 0.002565 ms | 2.7458x |
| 1000 rows / delete 50 rows | 0.029355 ms | 0.016511 ms | 1.7779x |
| 10000 rows / delete 500 rows | 0.270587 ms | 0.165320 ms | 1.6367x |
