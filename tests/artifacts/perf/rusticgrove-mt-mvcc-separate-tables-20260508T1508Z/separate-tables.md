# mt-mvcc-bench Summary

- Workload shape: `separate_tables`
- Rows per thread: `250`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/rusticgrove-mt-mvcc-separate-tables-20260508T1508Z/history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 272106 | 346283 | 0.79x | 0.92 | 0.72 | 1.27x | 0 | 0 |
| 2 | 442724 | 288763 | 1.53x | 1.13 | 1.73 | 0.65x | 0 | 0 |
| 4 | 750970 | 98581 | 7.62x | 1.33 | 10.14 | 0.13x | 0 | 0 |
| 8 | 720938 | 24880 | 28.98x | 2.77 | 80.39 | 0.03x | 0 | 0 |
