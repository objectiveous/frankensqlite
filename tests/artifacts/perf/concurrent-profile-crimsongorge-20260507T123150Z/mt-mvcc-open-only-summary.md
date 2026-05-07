# mt-mvcc-bench Summary

- Rows per thread: `0`
- Iterations: `10`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/tmp/frankensqlite-concurrent-profile-history-crimsongorge-openonly.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 0 | 0 | 0.00x | 0.64 | 0.47 | 1.36x | 0 | 0 |
| 4 | 0 | 0 | 0.00x | 0.84 | 1.01 | 0.83x | 0 | 0 |
| 8 | 0 | 0 | 0.00x | 1.37 | 1.16 | 1.18x | 0 | 0 |
