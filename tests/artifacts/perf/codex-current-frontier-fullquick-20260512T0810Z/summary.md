# Current Frontier Full-Quick Benchmark

- Date: 2026-05-12 08:18:17 UTC
- Command: `CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --no-html --json-out tests/artifacts/perf/codex-current-frontier-fullquick-20260512T0810Z/full.json`
- Source commit: `6d26e7d50c6e99137c7c451f6d1e03111fd1cacf`
- Worktree state recorded by artifact: clean
- Build profile: `release-perf`

## Aggregate

| Metric | Value |
|---|---:|
| Total scenarios | `93` |
| FrankenSQLite faster / comparable / C SQLite faster | `80 / 3 / 10` |
| Geomean F/C time ratio | `0.27423740495932974` |
| Median F/C time ratio | `0.31399627931563806` |
| Average F/C time ratio | `0.4852381197057886` |
| p90 F/C time ratio | `1.053904679087915` |
| p99 F/C time ratio | `2.969811320754717` |
| Per-category weighted score | `0.37100042867198824` |

## Per Category

| Category | n | Geomean F/C | Median F/C | p90 F/C |
|---|---:|---:|---:|---:|
| read_aggregate | 25 | `0.0807830613938502` | `0.12346278730856748` | `0.5097386844111735` |
| mixed | 1 | `0.19301824588234973` | `0.19301824588234973` | `0.19301824588234973` |
| read_single | 33 | `0.21356845554749707` | `0.21825275062464383` | `0.319965800995914` |
| concurrent_writers | 3 | `0.8349139182428391` | `1.053904679087915` | `1.087960816497609` |
| write_bulk | 22 | `0.7762692530288624` | `0.7452274601429357` | `1.0804324969452417` |
| write_single | 9 | `1.1532331569033791` | `0.9096989966555183` | `2.969811320754717` |

## UPDATE/DELETE Rows

| Scenario | C median ms | F median ms | F/C |
|---|---:|---:|---:|
| 100 rows / update 10 rows | `0.004288` | `0.006041` | `1.4088152985074627` |
| 100 rows / delete 5 rows | `0.002385` | `0.007083` | `2.969811320754717` |
| 1000 rows / update 100 rows | `0.037089` | `0.028583` | `0.7706597643506161` |
| 1000 rows / delete 50 rows | `0.01603` | `0.029605` | `1.846849656893325` |
| 10000 rows / update 1000 rows | `0.364462` | `0.246722` | `0.6769484884569584` |
| 10000 rows / delete 500 rows | `0.162635` | `0.264455` | `1.6260645002613214` |

## Mixed OLTP

| Scenario | C median ms | F median ms | F/C |
|---|---:|---:|---:|
| 5K ops (80r/20w) on 5K-row table | `223.28067900000002` | `43.097245` | `0.19301824588234973` |
