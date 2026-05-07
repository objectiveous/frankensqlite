# mt-mvcc-bench Summary

- Rows per thread: `10`
- Iterations: `20`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/tmp/frankensqlite-concurrent-profile-history-crimsongorge-2t-10row.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 10280 | 12462 | 0.82x | 1.95 | 1.60 | 1.21x | 0 | 0 |
