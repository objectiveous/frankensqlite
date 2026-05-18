# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `1000`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `failed` (threshold `5.00%`, history `.bench-history/mt-mvcc-bench.latest.json`)
- Regressions:
  - 16 threads: 7.87x -> 5.37x (31.80% drop)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 16 | 161243 | 30043 | 5.37x | 99.23 | 532.57 | 0.19x | 0 | 0 |
