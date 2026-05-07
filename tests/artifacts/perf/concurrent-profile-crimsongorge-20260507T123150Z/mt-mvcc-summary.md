# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `5`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/tmp/frankensqlite-concurrent-profile-history-crimsongorge.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 435899 | 944270 | 0.46x | 4.59 | 2.12 | 2.17x | 0 | 0 |
| 4 | 347707 | 408145 | 0.85x | 11.50 | 9.80 | 1.17x | 0 | 0 |
| 8 | 280580 | 98889 | 2.84x | 28.51 | 80.90 | 0.35x | 0 | 0 |
