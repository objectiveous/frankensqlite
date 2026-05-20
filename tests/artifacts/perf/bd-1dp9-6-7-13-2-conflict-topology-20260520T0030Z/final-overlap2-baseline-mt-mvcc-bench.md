# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `300`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/bd-1dp9-6-7-13-2-conflict-topology-20260520T0030Z/final-overlap2-baseline-mt-mvcc-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 83963 | 306809 | 0.27x | 7.15 | 1.96 | 3.65x | 0 | 0 |
| 4 | 159173 | 125710 | 1.27x | 7.54 | 9.55 | 0.79x | 0 | 0 |
| 8 | 93459 | 29875 | 3.13x | 25.68 | 80.33 | 0.32x | 0 | 0 |
