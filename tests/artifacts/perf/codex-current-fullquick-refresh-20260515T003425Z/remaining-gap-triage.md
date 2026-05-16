# Remaining Gap Triage After Full-Quick Refresh

- Source artifact: `summary.md`
- Run timestamp from benchmark output: 2026-05-16 00:41:24 UTC.
- Source checkout: `06a37f61e0ad97ffa95449f2f97a27ea080c821c`

## Current Ranking

| Rank | Section | Scenario | F/C | Current conclusion |
|---:|---|---|---:|---|
| 1 | UPDATE/DELETE | 100 rows / delete 5 rows | 2.88x | DML mutation operator; retained-run micro-patches remain fenced |
| 2 | UPDATE/DELETE | 1000 rows / delete 50 rows | 1.81x | DML mutation operator |
| 3 | UPDATE/DELETE | 10000 rows / delete 500 rows | 1.61x | DML mutation operator |
| 4 | UPDATE/DELETE | 100 rows / update 10 rows | 1.39x | transaction/DML lifecycle fixed-cost tail |
| 5 | INSERT single txn small_3col | 100 rows | 1.32x | 100-row fixed-cost/fused builder boundary; not enough alone |
| 6 | INSERT transaction strategy small_3col | 100 rows / autocommit | 1.21x | 100-row fixed-cost/fused builder boundary |
| 7 | INSERT transaction strategy small_3col | 100 rows / batched (100/txn) | 1.17x | 100-row fixed-cost/fused builder boundary |
| 8 | INSERT single txn tiny_1col | 100 rows | 1.12x | high-variance 100-row fixed-cost tail |
| 9 | INSERT single txn large_10col | 100 rows | 1.10x | near-noise fixed-cost tail |
| 10 | INSERT single txn medium_6col | 100 rows | 1.09x | near-noise fixed-cost tail |
| 11 | INSERT single txn large_10col | 10000 rows | 1.09x | large-row construction tail below gate |
| 12 | INSERT transaction strategy small_3col | 100 rows / single txn | 1.08x | near-noise fixed-cost tail |
| 13 | INSERT record-size comparison | large_10col / 10000 rows | 1.03x | comparable/noise band |

## Delta From Older Gap Triage

The older gap triage was based on
`tests/artifacts/perf/codex-current-fullquick-1c7f5b33-20260515T002530Z/full-quick.json`
and reported low-thread concurrent-writer rows as red. This refresh reports
2-, 4-, and 8-writer rows as 4.21x, 3.28x, and 3.31x faster than C SQLite, so
concurrent-writer source work is not a current target from this matrix.

## Source-Work Ranking

| Lever | Covers rows | Score | Status |
|---|---|---:|---|
| Transaction-local DML mutation operator with grouped leaf flush | DELETE rows and possibly 100-row UPDATE fixed-cost tail | 3.0 | only source lever clearing the implementation gate |
| Fused row/body/page construction and MVCC publication for INSERT | 100-row INSERT tails and large-row construction tails | 1.8 | below gate until a sharper profile isolates a single helper |
| Low-thread concurrent-writer representation work | none in current matrix | 0.0 | not a current target |

The next source slice should still be proof-first DML mutation-operator work,
not a standalone INSERT or concurrent-writer micro-patch.
