# mt-mvcc-bench Summary

- Rows per thread: `1`
- Iterations: `20`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/tmp/frankensqlite-concurrent-profile-history-crimsongorge-2t-1row.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 1031 | 1247 | 0.83x | 1.94 | 1.60 | 1.21x | 0 | 0 |
