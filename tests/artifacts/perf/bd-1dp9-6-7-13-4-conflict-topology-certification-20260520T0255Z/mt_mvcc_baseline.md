# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `300`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/projects/frankensqlite/tests/artifacts/perf/bd-1dp9-6-7-13-4-conflict-topology-certification-20260520T0255Z/mt_mvcc_baseline.history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 8 | 83363 | 40012 | 2.08x | 28.79 | 59.98 | 0.48x | 0 | 0 |
