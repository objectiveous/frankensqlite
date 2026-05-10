# DML delete subphase profile: 2026-05-10

## Scenario

- Command: `FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-delete-subphase-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/codex-dml-delete-subphase-20260510T234045Z/update-profile.json --no-html`
- Base commit before profiling delta: `d3d641fc4b6a58801fbcbfb15f0306ae9f50e54f`
- Working-tree delta: direct DELETE profiling counters in `crates/fsqlite-core/src/connection.rs`, surfaced by `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`.
- Toolchain: `rustc 1.97.0-nightly (82bee9650 2026-05-09)`, `cargo 1.97.0-nightly (a343accce 2026-05-08)`
- Purpose: split direct DELETE timing into quotient-filter consultation, B-tree seek, and physical fallback delete, without changing DML behavior.

## Result

| Workload | C median | F median | F/C | qf ns | seek ns | physical ns | leaf start ns | leaf active ns | leaf flush ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 0.002354 ms | 0.008456 ms | 3.592x | 50 | 1,032 | 0 | 401 | 530 | 1,923 |
| 1000 rows / delete 50 rows | 0.016230 ms | 0.034294 ms | 2.113x | 160 | 5,171 | 0 | 1,532 | 5,356 | 14,166 |
| 10000 rows / delete 500 rows | 0.164017 ms | 0.316061 ms | 1.927x | 1,622 | 38,092 | 14,396 | 10,885 | 50,726 | 104,381 |

The new counters reject quotient-filter consultation and physical fallback delete as the primary DELETE gap. The 500-row case still spends most attributable DELETE subphase time in leaf-run mutation and dirty-run flush (`166.0 us` combined), with seek secondary (`38.1 us`) and physical fallback only `14.4 us`.

## Files

- `update-profile.json`: benchmark rows.
- `stderr.txt`: benchmark log with `dml_profile` lines.
- `stdout.txt`: rendered benchmark table.
