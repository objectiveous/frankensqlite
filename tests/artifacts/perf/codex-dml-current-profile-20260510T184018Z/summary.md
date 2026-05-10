# Current-Source UPDATE/DELETE Profile

Date: 2026-05-10 UTC

Source: `8341c4fa7b5ee3cce2c69569961f03f676fe6f95`

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-codex-after-63cf-current-target/release-perf/comprehensive-bench \
  --quick --filter UPDATE \
  --json-out tests/artifacts/perf/codex-dml-current-profile-20260510T184018Z/update-delete-profile.json \
  --no-html
```

Raw stdout and stderr are retained as `stdout.log` and `stderr.log`.

## Summary

| Metric | Value |
| --- | ---: |
| Total scenarios | 6 |
| FrankenSQLite faster | 2 |
| Comparable | 0 |
| C SQLite faster | 4 |
| Geomean ratio | 1.57221 |
| p90 ratio | 3.45938 |
| p99 ratio | 3.45938 |

Lower ratios are better for FrankenSQLite.

## Rows

| Ratio | Scenario | C median ms | F median ms | C CV % | F CV % |
| ---: | --- | ---: | ---: | ---: | ---: |
| 3.45938 | 100 rows / delete 5 rows | 0.002314 | 0.008005 | 8.11 | 8.51 |
| 2.08647 | 1000 rows / delete 50 rows | 0.015889 | 0.033152 | 2.17 | 2.71 |
| 1.99895 | 10000 rows / delete 500 rows | 0.161171 | 0.322173 | 9.30 | 12.98 |
| 1.45064 | 100 rows / update 10 rows | 0.004558 | 0.006612 | 5.10 | 4.33 |
| 0.85932 | 1000 rows / update 100 rows | 0.037460 | 0.032190 | 0.93 | 1.74 |
| 0.83973 | 10000 rows / update 1000 rows | 0.381434 | 0.320300 | 9.58 | 37.86 |

## Profile Attribution

| Row | mutate us | commit us | direct UPDATE | direct DELETE | delete leaf starts | active hits | active misses | leaf flushes | leaf flush ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| update 10/100 | 6.7 | 8.3 | 10 | 0 | 0/0 | 0/0 | 0 | 0/0 | 0 |
| delete 5/100 | 4.1 | 8.1 | 0 | 5 | 1/1 | 4/4 | 0 | 1/1 | 2,154 |
| update 100/1000 | 36.9 | 9.3 | 100 | 0 | 0/0 | 0/0 | 0 | 0/0 | 0 |
| delete 50/1000 | 35.2 | 10.4 | 0 | 50 | 6/6 | 44/49 | 5 | 6/6 | 14,247 |
| update 1000/10000 | 336.4 | 34.8 | 1000 | 0 | 0/0 | 0/0 | 0 | 0/0 | 0 |
| delete 500/10000 | 314.8 | 36.8 | 0 | 500 | 64/67 | 433/496 | 63 | 64/64 | 106,756 |

Every profiled UPDATE/DELETE stayed on the dedicated prepared direct DML path
(`fast == mutations`, `slow == 0`). The 100-row DELETE case has no active-run
misses and only one dirty leaf-run flush, matching the earlier tiny-delete
isolation result: the gap is fixed ceremony/publication rather than a missed
row-level fast path.

## Decision

No source patch was attempted from this profile. The nearby standalone levers
are already measured negative families in the ledger: direct DML route-check
hoisting, statement-reuse tracing-gate caching, scratch/lookaside guard
removal, exact transaction-control fast paths, and wrapper-only direct flush
reshaping.

The remaining credible DML shape is still a broader retained direct-DML
execution design that removes cursor/root-descent and route ceremony together
while protecting savepoints, rollback, read-your-writes, and full-quick primary
score in the same measurement window.
