# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `1000`
- Iterations: `1`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/codex-shared16-mtmvcc-20260510T195814Z/history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 16 | 247633 | 25275 | 9.80x | 64.61 | 633.03 | 0.10x | 0 | 0 |
