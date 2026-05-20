# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `300`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/projects/frankensqlite/tests/artifacts/perf/bd-1dp9-6-7-13-4-conflict-topology-certification-20260520T0145Z/mt_mvcc_enforced.history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 8 | 47258 | 24626 | 1.92x | 50.78 | 97.46 | 0.52x | 0 | 0 |
