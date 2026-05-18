# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `1000`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `passed` (threshold `5.00%`, history `.bench-history/mt-mvcc-bench.latest.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 16 | 214399 | 30028 | 7.14x | 74.63 | 532.83 | 0.14x | 0 | 0 |
