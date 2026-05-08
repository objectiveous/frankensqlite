# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `10`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/projects/frankensqlite/tests/artifacts/perf/windyibis-current-frontier-repeat-20260508T091105Z/clean-mt-mvcc-2t-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 490985 | 885208 | 0.55x | 4.07 | 2.26 | 1.80x | 0 | 0 |
