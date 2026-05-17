# INSERT profile refresh - 2026-05-17T0900Z

## Command

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_INSERT=1 CARGO_TARGET_DIR=/data/tmp/frankensqlite-insert-profile-refresh-20260517 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-insert-profile-refresh-20260517T0900Z/insert.json --no-html
```

RCH reported no admissible workers and ran locally.

## Matrix Summary

- Total scenarios: 25
- FrankenSQLite faster / comparable / C SQLite faster: 15 / 3 / 7
- Average F/C ratio: 0.9007576347598318
- Geomean F/C ratio: 0.8757887703830092
- P90 / P99 F/C ratio: 1.1133012706610754 / 1.3520734942097432
- Weighted score: 0.9735592077116765

## Red Rows Over 1.05x

| Scenario | Category | C SQLite ms | FrankenSQLite ms | F/C | C CV | F CV |
|---|---:|---:|---:|---:|---:|---:|
| 100 rows | write_bulk | 0.077285 | 0.104495 | 1.352073 | 12.61 | 37.36 |
| 100 rows | write_bulk | 0.182782 | 0.202529 | 1.108036 | 32.45 | 12.92 |
| 100 rows | write_bulk | 0.253254 | 0.281948 | 1.113301 | 26.29 | 25.24 |
| 10000 rows | write_bulk | 10.000202 | 10.805179 | 1.080496 | 4.74 | 6.52 |
| 100 rows / autocommit | write_single | 0.169498 | 0.209252 | 1.234540 | 25.08 | 29.62 |
| 100 rows / single txn | write_bulk | 0.138850 | 0.151674 | 1.092359 | 19.90 | 26.14 |
| large_10col -- 10 cols (~600B: includes long text fields) | write_bulk | 10.205106 | 11.284226 | 1.105743 | 6.36 | 13.09 |

## Hotspot Table

The fixed-cost 100-row rows have high variance and are not a clean source target by themselves. The durable source target is the large 10K record-size row, which also reproduced in the non-profile repeat.

| Rank | Location | Profile Signal | Value | Interpretation |
|---:|---|---:|---:|---|
| 1 | prepared direct INSERT row/body construction | `row_build_ns` | 20549477 | Dominant profiled F-side work, but inflated by nested `Instant` probes. |
| 2 | direct preserialized record construction | `preserialize_ns` | 19952566 | Same region as row build; existing no-retry fences cover standalone serializer/template/scratch tweaks. |
| 3 | expression evaluation | `preserialize_eval_ns` | 4207683 | Large-row concat/arithmetic still visible; standalone param-literal specializations are rejected. |
| 4 | page-run flush / memory publication | `direct_flush_ns` | 2837823 | Page-run/page-publication work remains material. |
| 5 | B-tree empty-root bulk leaf build/write | `btree_bulk_leaf_build/write` | 2000/1099301, 2000/184767 | B-epsilon-style bulk loader is active; page construction remains part of the broad fused-design target. |

## Decision

No optimization patch was attempted from this artifact. The current profile confirms that the red large-row INSERT gap is a fused row/body/page construction problem, not a standalone affinity, concat, scratch, page-run arena, or leaf-writer micro-problem. The next code candidate must remove duplicate row construction and page construction together, then pass both focused INSERT and full quick weighted-score gates.
