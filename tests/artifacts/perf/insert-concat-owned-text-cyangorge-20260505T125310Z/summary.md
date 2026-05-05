# Direct INSERT concat owned-text candidate

Run: `2026-05-05T12:53:10Z`

Candidate:

- In `crates/fsqlite-core/src/connection.rs`, long concat results moved the
  reusable `String` scratch into `SmallText::from_string` instead of copying
  `text_scratch.as_str()` into a second heap string.
- Inline-size concat results kept the existing inline `SmallText` path.

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-concat-owned-text-cyangorge-20260505T125310Z/report.json --no-html
```

Summary:

- `total_scenarios`: 25
- `geomean_ratio`: `2.5245x`
- `per_category_weighted.score`: `1.7467`
- `p99_ratio`: `4.4258x`
- `write_bulk.geomean_ratio`: `2.7079x`
- `write_single.geomean_ratio`: `1.5092x`

Verdict:

Rejected and reverted. The candidate avoided a copy but destroyed reusable
scratch capacity, causing repeated hot-path allocation and broad insert
regression.

Target rows:

- `large_10col` single-transaction 10K: FrankenSQLite `43.055 ms`, C SQLite `9.908 ms`, ratio `4.3456x`
- record-size `large_10col` 10K: FrankenSQLite `41.902 ms`, C SQLite `9.468 ms`, ratio `4.4258x`
