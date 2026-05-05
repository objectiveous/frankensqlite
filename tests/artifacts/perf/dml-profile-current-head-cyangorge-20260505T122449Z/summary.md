# Current-head DML profile

Run: `2026-05-05T12:24:49Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/dml-profile-current-head-cyangorge-20260505T122449Z/report.json --no-html
```

Git SHA: `237261d2`

Summary:

- `total_scenarios`: 6
- `geomean_ratio`: `2.8737x`
- `per_category_weighted.score`: `2.8737`

Interpretation:

The apparent UPDATE/DELETE gap is largely setup-driven in this benchmark
slice. For 10K-row update, the profile showed `setup_us=6799.8`,
`mutate_us=1155.9`, and `commit_us=820.8`. For 10K-row delete,
`setup_us=6394.6`, `mutate_us=834.7`, and `commit_us=696.9`.
That points back to insert/setup throughput rather than a clean DML-only
mutation bottleneck.
