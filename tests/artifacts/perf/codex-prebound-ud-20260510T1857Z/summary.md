# Prebound UPDATE/DELETE Publication Candidate

Date: 2026-05-10 UTC

Source baseline: `836527fa6fae0a646abdf0b042bf28b5c0d22d29` plus a rejected local source patch.

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-codex-prebound-ud-target/release-perf/comprehensive-bench \
  --quick --filter UPDATE \
  --json-out tests/artifacts/perf/codex-prebound-ud-20260510T1857Z/dml-update-quick.json \
  --no-html
```

Raw stdout and stderr are retained as `stdout.log` and `stderr.log`.

## Candidate

Thread the existing `PreparedDmlEntryProof.publication` into
`try_execute_precompiled_prepared_update_delete_autocommit_direct_simple_fast`
instead of calling `ensure_autocommit_txn_with_publication_hint(..., None)`.

The focused regression test proved the intended micro-effect: stale file-backed
prepared direct UPDATE and DELETE each used one pager publication refresh instead
of rebinding during autocommit begin. The source patch was then unwound because
the DML benchmark did not improve.

## Summary

| Metric | Value |
| --- | ---: |
| Total scenarios | 6 |
| FrankenSQLite faster | 2 |
| Comparable | 0 |
| C SQLite faster | 4 |
| Geomean ratio | 1.60098 |
| p90 ratio | 3.62947 |
| p99 ratio | 3.62947 |

Lower ratios are better for FrankenSQLite. The current-source baseline in
`tests/artifacts/perf/codex-dml-current-profile-20260510T184018Z/` had geomean
`1.57221`, p90 `3.45938`, and p99 `3.45938`.

## Rows

| Ratio | Scenario | C median ms | F median ms |
| ---: | --- | ---: | ---: |
| 3.62947 | 100 rows / delete 5 rows | 0.002294 | 0.008326 |
| 2.14997 | 1000 rows / delete 50 rows | 0.015690 | 0.033733 |
| 2.02086 | 10000 rows / delete 500 rows | 0.156583 | 0.316432 |
| 1.59108 | 100 rows / update 10 rows | 0.004238 | 0.006743 |
| 0.85958 | 1000 rows / update 100 rows | 0.036889 | 0.031709 |
| 0.78078 | 10000 rows / update 1000 rows | 0.358251 | 0.279714 |

## Decision

Rejected and source-unwound. Removing the duplicate publication bind was real
but too small/noisy relative to the DML row costs, and this same-window focused
matrix worsened the DML geomean and tail ratios versus the current-source
profile.
