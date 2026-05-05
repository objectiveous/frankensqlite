# PageData shared-pair quick-balance rerun

Run: `2026-05-05T12:16:51Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --filter insert --json-out tests/artifacts/perf/insert-pagedata-shared-pair-rerun-cyangorge-20260505T121651Z/report.json --no-html
```

Baseline: `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/`

Verdict: rejected and reverted.

The rerun confirmed the aggregate ratio improvement but also confirmed the
targeted large-row regression:

- `per_category_weighted.score`: `1.7141 -> 1.6914`
- `geomean_ratio`: `2.3519x -> 2.1634x`
- `large_10col` single transaction 10K: `34.756 ms -> 38.651 ms`
- `large_10col` single transaction 100K: `415.902 ms -> 444.772 ms`

Do not repeat the shared-pair shape. It trades one clone at split time for
copy-on-write when the rightmost leaf is immediately reused.

