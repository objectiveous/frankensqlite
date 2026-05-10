# Current-Source INSERT Red-Row Repeat

Date: 2026-05-10 UTC

Source: `2ffbbd31a8bc8b12f5398f2686a2430eb2428c8c`

Command:

```bash
/data/tmp/frankensqlite-codex-after-63cf-current-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-insert-red-repeat-20260510T183336Z/insert-repeat1.json \
  --no-html
```

The same binary was run two more times as `insert-repeat2.json` and
`insert-repeat3.json`. The profile-hook pass used:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-codex-after-63cf-current-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-insert-red-repeat-20260510T183336Z/insert-profile.json \
  --no-html
```

Raw stdout/stderr for each run is retained in this directory.

## Repeat Summary

| Run | Faster | Comparable | C faster | Geomean | p90 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `insert-repeat1.json` | 17 | 1 | 7 | 0.85408 | 1.14949 | 1.16660 |
| `insert-repeat2.json` | 17 | 2 | 6 | 0.81140 | 1.11263 | 1.13059 |
| `insert-repeat3.json` | 17 | 1 | 7 | 0.86149 | 1.15370 | 1.17756 |

## Repeated Red Rows

| Row | Repeat 1 | Repeat 2 | Repeat 3 | Profile run |
| --- | ---: | ---: | ---: | ---: |
| `small_3col` 100 rows, single txn | 1.12455 | 1.11977 | 1.15370 | 1.09473 |
| `small_3col` 100 rows, strategy single txn | 1.16404 | 1.11263 | 1.17081 | 1.06065 |
| `small_3col` 100 rows, strategy batched | 1.16660 | 1.09190 | 1.17756 | 1.09345 |
| `large_10col` 100 rows, single txn | 1.10287 | 1.13059 | 1.13742 | 1.05349 |
| `large_10col` 10K rows, single txn | 1.14949 | 1.10570 | 1.13786 | 1.08072 |
| `large_10col` 10K rows, record size | 1.12693 | 1.07378 | 1.13842 | 1.03774 |
| `medium_6col` 100 rows, single txn | 1.06766 | 1.02506 | 1.10718 | 1.05983 |
| `tiny_1col` 100 rows, single txn | 1.03884 | 1.04643 | 1.01119 | 1.21997 |

The `tiny_1col` profile-row ratio is not actionable because both engines were
high-variance in that profile pass (`C CV 144.8%`, `F CV 50.0%`).

## Profile Attribution

Representative `FSQLITE_BENCH_PROFILE_INSERT=1` counters:

| Row | setup us | begin us | prepare us | insert us | commit us | row-build ns | direct-flush ns | page-run storage | page-pool misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: |
| `small_3col` 100 single | 16.6 | 10.4 | 9.8 | 56.0 | 11.2 | 11,133 | 4,549 | arena | 0 |
| `medium_6col` 100 single | 18.0 | 10.7 | 11.8 | 65.3 | 21.4 | 20,150 | 9,939 | arena | 5 |
| `large_10col` 100 single | 20.2 | 11.2 | 17.4 | 88.2 | 47.3 | 40,184 | 24,186 | owned | 20 |
| `large_10col` 10K single | 47.1 | 15.8 | 35.4 | 8,813.9 | 5,379.0 | 4,052,787 | 2,994,797 | owned | 2,004 |
| `large_10col` 10K record-size | 48.5 | 16.7 | 35.3 | 8,948.3 | 5,593.1 | 4,095,196 | 3,227,152 | owned | 2,004 |

All listed rows stayed on the prepared direct INSERT fast path
(`direct_insert == fast`, `slow == 0`) and used the empty-root page-run bulk-load
path (`page_run_empty_root=1`, `page_run_fallbacks=0`).

## Decision

No source patch was attempted. The repeated rows are real enough to watch, but
the apparent source levers are already fenced by the ledger: standalone
concat/param-one/record-template row-build variants, direct page-run
threshold/arena changes, arena-only large-record page-run buffering, borrowed
owned-record flushing, and prebuilt empty-root leaf builders all failed earlier
focused or full-quick gates.

The credible next source shape is still the broader fused record-body and
page-layout builder that constructs records and B-tree pages together, then
proves focused INSERT and full-quick primary-score movement in the same
measurement window. The 100-row rows are too fixed-cost-heavy for another
standalone micro-optimization.
