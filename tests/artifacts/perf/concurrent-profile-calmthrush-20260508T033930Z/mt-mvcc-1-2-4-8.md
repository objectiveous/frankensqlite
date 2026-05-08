# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `8`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/concurrent-profile-calmthrush-20260508T033930Z/mt-mvcc-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 685182 | 1189899 | 0.58x | 1.46 | 0.84 | 1.73x | 0 | 0 |
| 2 | 612800 | 835121 | 0.73x | 3.27 | 2.40 | 1.36x | 0 | 0 |
| 4 | 384945 | 413332 | 0.93x | 10.39 | 9.68 | 1.07x | 0 | 0 |
| 8 | 313238 | 99188 | 3.16x | 25.67 | 80.65 | 0.32x | 0 | 0 |
