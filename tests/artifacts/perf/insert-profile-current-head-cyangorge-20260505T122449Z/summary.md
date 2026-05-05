# Current-head insert profile

Run: `2026-05-05T12:24:49Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json --no-html
```

Git SHA: `237261d2`

Summary:

- `total_scenarios`: 25
- `geomean_ratio`: `2.3623x`
- `per_category_weighted.score`: `1.6991`
- `write_bulk` geomean: `2.5153x`
- `write_single` geomean: `1.4908x`

Hot large-row profile evidence:

- `large_10col` single transaction 10K: F median `36.165 ms`; profile showed
  `row_build_ns=5.958 ms`, `btree_insert_ns=7.311 ms`,
  `btree_quick_balance_ns=3.447 ms`, and `commit_roundtrip_ns=17.036 ms`.
- record-size `large_10col` 10K: F median `37.056 ms`; profile showed
  `row_build_ns=5.973 ms`, `btree_insert_ns=8.452 ms`,
  `btree_quick_balance_ns=4.478 ms`, and `commit_roundtrip_ns=15.957 ms`.
