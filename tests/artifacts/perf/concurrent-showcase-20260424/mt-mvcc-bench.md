# mt-mvcc-bench Summary

- Rows per thread: `250`
- Iterations: `1`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v1`

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 4 | 3178 | 212016 | 0.01x | 314.68 | 4.72 | 66.72x | 0 | 0 |
| 8 | 6125 | 36152 | 0.17x | 326.55 | 55.32 | 5.90x | 0 | 0 |
| 16 | 5181 | 6325 | 0.82x | 772.04 | 632.44 | 1.22x | 0 | 0 |
