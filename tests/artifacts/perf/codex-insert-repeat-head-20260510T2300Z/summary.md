# Current INSERT Repeat Profile

Date: 2026-05-10 UTC

Source: `6e40d540dc1bd620cce71b17b6fdd7bb3adc66e4`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-codex-dml-profile-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/codex-insert-repeat-head-20260510T2300Z/insert-profile.json \
  --no-html
```

The benchmark binary was built before `6e40d540`, but that commit changed only
documentation and perf artifacts; no Rust source changed between the build and
this run.

`insert-profile.json` reports `git_dirty: true`; the visible workspace dirt at
the time was pre-existing untracked RCH target directories.

## Summary

| Metric | Value |
| --- | ---: |
| Total scenarios | 25 |
| FrankenSQLite faster | 17 |
| Comparable | 1 |
| C SQLite faster | 7 |
| Geomean ratio | 0.89248 |
| Weighted primary score | 0.87217 |

Lower ratios are better for FrankenSQLite.

## C-Faster Rows

| Ratio | Section | Scenario | C median ms | F median ms | C CV % | F CV % |
| ---: | --- | --- | ---: | ---: | ---: | ---: |
| 1.21687 | Single Transaction - large_10col | 10000 rows | 9.256635 | 11.264134 | 0.75 | 5.30 |
| 1.21548 | Record Size Comparison | large_10col - 10 cols | 9.561746 | 11.622095 | 0.88 | 6.95 |
| 1.20190 | Single Transaction - small_3col | 100 rows | 0.073047 | 0.087795 | 4.96 | 8.35 |
| 1.20182 | Transaction Strategy Comparison | 100 rows / batched (100/txn) | 0.073668 | 0.088536 | 6.61 | 3.55 |
| 1.18650 | Transaction Strategy Comparison | 100 rows / single txn | 0.073217 | 0.086872 | 2.80 | 6.41 |
| 1.12842 | Single Transaction - medium_6col | 100 rows | 0.100568 | 0.113483 | 3.44 | 5.28 |
| 1.11270 | Single Transaction - large_10col | 100 rows | 0.155310 | 0.172814 | 20.81 | 4.31 |

## Profile Notes

The large 10-column 10K record-size profile stayed on the prepared direct
INSERT path (`direct_insert=10000`, `fast=10000`, `slow=0`) and used the
empty-root page-run path (`page_run_flushes=1`, `page_run_owned=1`,
`page_run_empty_root=1`, `page_run_fallbacks=0`). Its visible cost was split
between row construction (`row_build_ns=4619953`), direct page-run flush
publication (`direct_flush_ns=3071141`), B-tree insertion (`btree_insert_ns=818128`),
and commit (`commit_us=5221.8`).

This repeats the prior INSERT red-row source screen with a worse but still
consistent large-row tail. The fenced standalone families remain the same:
concat/record serialization variants, direct page-run threshold or arena
changes, borrowed owned-record flushing, and empty-root leaf layout caching.
