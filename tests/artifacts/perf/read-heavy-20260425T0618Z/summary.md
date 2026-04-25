# Read-Heavy Benchmark

- Bead: `bd-db300.7.1.2`
- Run: `read-heavy-20260425T0618Z`
- Rows: `10000`
- Reads/thread: `10000`
- Build profile: `release-perf`

| Threads | FrankenSQLite reads/sec | SQLite reads/sec | Ratio |
|---:|---:|---:|---:|
| 1 | 18655 | 286636 | 0.07x |
| 2 | 50247 | 681380 | 0.07x |
| 4 | 101908 | 513119 | 0.20x |
| 8 | 162232 | 494384 | 0.33x |
