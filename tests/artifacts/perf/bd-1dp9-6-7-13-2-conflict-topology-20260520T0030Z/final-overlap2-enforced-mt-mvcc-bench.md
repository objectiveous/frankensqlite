# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `300`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/bd-1dp9-6-7-13-2-conflict-topology-20260520T0030Z/final-overlap2-enforced-mt-mvcc-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 201631 | 269809 | 0.75x | 2.98 | 2.22 | 1.34x | 0 | 0 |
| 4 | 141595 | 113749 | 1.24x | 8.47 | 10.55 | 0.80x | 0 | 0 |
| 8 | 111700 | 42551 | 2.63x | 21.49 | 56.40 | 0.38x | 0 | 0 |
