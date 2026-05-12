# mt-mvcc-bench Summary

- Workload shape: `shared_table`
- Rows per thread: `1000`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v3`

- Pass-over-pass gate: `passed` (threshold `5.00%`, history `tests/artifacts/perf/codex-16-thread-shared-verify-20260512T1128Z/history-after-fix-16t-1000r-iters3.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 16 | 226357 | 25278 | 8.95x | 70.68 | 632.96 | 0.11x | 0 | 0 |
