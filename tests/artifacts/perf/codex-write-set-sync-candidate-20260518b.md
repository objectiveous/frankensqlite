# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `100`
- Iterations: `1`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `.bench-history/mt-mvcc-bench.latest.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 16 | 60710 | 8820 | 6.88x | 26.35 | 181.41 | 0.15x | 0 | 0 |
