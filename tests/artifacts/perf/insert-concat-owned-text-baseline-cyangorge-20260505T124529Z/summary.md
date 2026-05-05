# Direct INSERT concat owned-text baseline

Run: `2026-05-05T12:45:29Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-concat-owned-text-baseline-cyangorge-20260505T124529Z/report.json --no-html
```

Summary:

- `total_scenarios`: 25
- `geomean_ratio`: `2.2471x`
- `per_category_weighted.score`: `1.6366`
- `p99_ratio`: `3.7572x`
- `write_bulk.geomean_ratio`: `2.3870x`
- `write_single.geomean_ratio`: `1.4431x`

Target rows:

- `large_10col` single-transaction 10K: FrankenSQLite `35.292 ms`, C SQLite `9.646 ms`, ratio `3.6589x`
- record-size `large_10col` 10K: FrankenSQLite `36.379 ms`, C SQLite `9.936 ms`, ratio `3.6611x`

This baseline was rebuilt from the reverted/current source immediately before
the concat owned-text candidate.
