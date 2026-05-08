# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `10`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `passed` (threshold `5.00%`, history `.bench-history/mt-mvcc-bench.latest.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 620024 | 942004 | 0.66x | 3.24 | 2.12 | 1.52x | 0 | 0 |
