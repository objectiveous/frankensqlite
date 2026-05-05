# Current-head full quick matrix

Run: `2026-05-05T12:24:49Z`

Command:

```bash
/data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/full-quick-current-head-cyangorge-20260505T122449Z/report.json --no-html
```

Git SHA: `237261d2`

Summary:

- `total_scenarios`: 93
- `franken_faster`: 58
- `csqlite_faster`: 35
- `geomean_ratio`: `0.4467x`
- `per_category_weighted.score`: `0.5658`

Remaining slow categories are write-heavy:

- `write_bulk` geomean `2.3562x`, p99 `3.8403x`
- `write_single` geomean `2.0563x`, p99 `3.0963x`
- `concurrent_writers` geomean `1.1514x`

The largest ratio rows were large-record INSERTs and small UPDATE/DELETE rows.
The DML profile showed setup dominates update/delete, so the next investigated
lever stayed on direct INSERT.
