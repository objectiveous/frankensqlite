# Current Full-Quick Refresh at 32b58eb

- Date: 2026-05-12 22:05 UTC.
- Commit: `32b58eb04b5e044ea6aa38341313d1ad45f39774`.
- Command: `comprehensive-bench --quick --no-html`.
- Artifact: `full-quick.json`.
- Worktree: clean; `benchmark_binary_older_than_git_head=false`.

## Headline

| Metric | Value |
|---|---:|
| Scenarios | `93` |
| FrankenSQLite faster / comparable / C SQLite faster | `80 / 3 / 10` |
| Average F/C time ratio | `0.4975749120x` |
| Geomean F/C time ratio | `0.2788127182x` |
| Median F/C time ratio | `0.2926984531x` |
| p90 F/C time ratio | `1.0667538706x` |
| p99 F/C time ratio | `3.0471537808x` |
| Per-category weighted score | `0.3751289346` |

## Remaining Rows Above Parity

| Section | Row | F ms | C ms | F/C |
|---|---|---:|---:|---:|
| INSERT single txn small_3col | 100 rows | `0.085701` | `0.076913` | `1.1142589679x` |
| INSERT single txn medium_6col | 100 rows | `0.106940` | `0.099847` | `1.0710386892x` |
| INSERT single txn large_10col | 100 rows | `0.167053` | `0.149209` | `1.1195906413x` |
| INSERT batched 100/txn | 100 rows | `0.106780` | `0.094316` | `1.1321514907x` |
| INSERT strategy single txn | 100 rows | `0.087914` | `0.074890` | `1.1739083990x` |
| INSERT record-size 10K large_10col | 10K rows | `9.684349` | `9.078335` | `1.0667538706x` |
| Concurrent writers | 2 writers x 1000 rows | `13.911872` | `13.441732` | `1.0349761474x` |
| Concurrent writers | 4 writers x 1000 rows | `20.651097` | `20.257951` | `1.0194069973x` |
| UPDATE/DELETE | 100 rows / update 10 rows | `0.006242` | `0.004318` | `1.4455766559x` |
| UPDATE/DELETE | 100 rows / delete 5 rows | `0.007173` | `0.002354` | `3.0471537808x` |
| UPDATE/DELETE | 1000 rows / delete 50 rows | `0.029636` | `0.015669` | `1.8913778799x` |
| UPDATE/DELETE | 10000 rows / delete 500 rows | `0.262762` | `0.157926` | `1.6638298950x` |

The current red tail remains the fenced families from the negative-results
ledger: small fixed-cost INSERT rows, low-thread shared-table concurrent rows,
and prepared-DML DELETE plus the tiny UPDATE row. The broader transaction-local
DML mutation/read-view operator is still the next unfenced source direction.
