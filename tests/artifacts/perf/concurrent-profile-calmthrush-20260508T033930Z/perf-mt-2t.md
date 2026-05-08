# mt-mvcc-bench Summary

- Rows per thread: `1000`
- Iterations: `80`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/concurrent-profile-calmthrush-20260508T033930Z/perf-mt-2t-history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 440434 | 768812 | 0.57x | 4.54 | 2.60 | 1.75x | 0 | 0 |
