# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `1000`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/codex-mt-shared-16-recheck-27d5f71d-20260512T2032Z-iters3/history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 16 | 226150 | 29997 | 7.54x | 70.75 | 533.38 | 0.13x | 0 | 0 |
