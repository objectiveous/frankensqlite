# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `5`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/tmp/frankensqlite-concurrent-profile-history-crimsongorge-apples.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 471721 | 850932 | 0.55x | 4.24 | 2.35 | 1.80x | 0 | 0 |
| 4 | 385831 | 409642 | 0.94x | 10.37 | 9.76 | 1.06x | 0 | 0 |
| 8 | 283299 | 99109 | 2.86x | 28.24 | 80.72 | 0.35x | 0 | 0 |
