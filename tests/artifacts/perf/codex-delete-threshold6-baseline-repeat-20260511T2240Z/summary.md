# Compact DELETE Threshold-6 Baseline Repeat

- Source state: retained `COMPACT_DELETE_SINGLE_PASS_MIN = 6` from `88dcadc9`.
- Benchmark command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update`.
- Purpose: same-window baseline for the threshold-5 probe.

| Scenario | FSQLite median | C SQLite median | Ratio |
|---|---:|---:|---:|
| 100 rows / delete 5 rows | 0.007063 ms | 0.002285 ms | 3.0910x |
| 1000 rows / delete 50 rows | 0.028724 ms | 0.016180 ms | 1.7753x |
| 10000 rows / delete 500 rows | 0.260167 ms | 0.162535 ms | 1.6007x |
