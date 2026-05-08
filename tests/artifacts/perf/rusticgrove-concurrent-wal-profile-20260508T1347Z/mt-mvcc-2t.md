# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `12`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/mt-mvcc-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 439282 | 683369 | 0.64x | 4.55 | 2.93 | 1.56x | 0 | 0 |
