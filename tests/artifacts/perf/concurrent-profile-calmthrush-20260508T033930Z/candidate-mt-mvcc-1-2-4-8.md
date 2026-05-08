# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `8`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/concurrent-profile-calmthrush-20260508T033930Z/candidate-mt-mvcc-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 860157 | 771966 | 1.11x | 1.17 | 1.30 | 0.90x | 0 | 0 |
| 2 | 600381 | 857536 | 0.70x | 3.33 | 2.33 | 1.43x | 0 | 0 |
| 4 | 400171 | 396583 | 1.01x | 10.04 | 10.09 | 1.00x | 0 | 0 |
| 8 | 317946 | 98767 | 3.22x | 25.16 | 81.00 | 0.31x | 0 | 0 |
