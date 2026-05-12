# Current Full-Quick Refresh at ce2309a2

- Date: 2026-05-12 22:43 UTC.
- Commit: `ce2309a2d7c3bbfcfd82340276933d53725051df`.
- Command: `comprehensive-bench --quick --no-html`.
- Artifact: `full-quick.json`.
- Worktree: clean; `benchmark_binary_older_than_git_head=false`.

## Headline

| Metric | Value |
|---|---:|
| Scenarios | `93` |
| FrankenSQLite faster / comparable / C SQLite faster | `79 / 2 / 12` |
| Average F/C time ratio | `0.4940249739x` |
| Geomean F/C time ratio | `0.2743918095x` |
| Median F/C time ratio | `0.2941593148x` |
| p90 F/C time ratio | `1.0915301110x` |
| p99 F/C time ratio | `2.9695560254x` |
| Per-category weighted score | `0.3684659618` |

## Remaining Rows Above Parity

| Section | Row | F ms | C ms | F/C |
|---|---|---:|---:|---:|
| UPDATE/DELETE | 100 rows / delete 5 rows | `0.007023` | `0.002365` | `2.9695560254x` |
| UPDATE/DELETE | 1000 rows / delete 50 rows | `0.029635` | `0.015690` | `1.8887826641x` |
| UPDATE/DELETE | 10000 rows / delete 500 rows | `0.265447` | `0.162383` | `1.6346969818x` |
| UPDATE/DELETE | 100 rows / update 10 rows | `0.006041` | `0.004348` | `1.3893744250x` |
| Concurrent writers | 2 writers x 1000 rows | `13.436939` | `11.526313` | `1.1657621132x` |
| INSERT strategy single txn | 100 rows | `0.084969` | `0.073928` | `1.1493480143x` |
| INSERT batched 100/txn | 100 rows | `0.085801` | `0.074840` | `1.1464591128x` |
| INSERT single txn small_3col | 100 rows | `0.083917` | `0.074178` | `1.1312922969x` |
| INSERT single txn medium_6col | 100 rows | `0.108323` | `0.098775` | `1.0966641357x` |
| INSERT single txn large_10col | 100 rows | `0.158667` | `0.145362` | `1.0915301110x` |
| INSERT single txn tiny_1col | 100 rows | `0.071033` | `0.066003` | `1.0762086572x` |
| Concurrent writers | 4 writers x 1000 rows | `21.243230` | `19.760996` | `1.0750080613x` |

Compared with the previous `32b58eb` README source artifact, this post-fix run
improves the primary weighted score, geomean, average, and p99. The remaining
red tail is still the known prepared-DML DELETE family plus near-parity
fixed-cost INSERT and low-thread file-backed concurrent rows.
