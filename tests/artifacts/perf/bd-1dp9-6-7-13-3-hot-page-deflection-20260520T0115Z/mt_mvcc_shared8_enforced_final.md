# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `200`
- Iterations: `5`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/bd-1dp9-6-7-13-3-hot-page-deflection-20260520T0115Z/mt_mvcc_shared8_enforced_threshold64.history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 8 | 138435 | 19865 | 6.97x | 11.56 | 80.54 | 0.14x | 0 | 0 |
