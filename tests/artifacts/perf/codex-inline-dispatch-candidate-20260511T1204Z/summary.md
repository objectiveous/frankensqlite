# TransactionKind Hot-Method Force-Inline Retry

Date: 2026-05-11

Base commit: `d49f5ed64325d4a5ee78014d03bed4d64a9f956c`

Candidate: add `#[inline(always)]` to the already-specialized
`TransactionKind::{get_page, write_page_data, free_page}` `TransactionHandle`
methods.

The candidate was rejected and manually unwound. Focused
`FSQLITE_BENCH_PROFILE_DML=1 --quick --filter update` repeats did not produce a
stable DELETE win, and the repeat worsened the 50-row and 500-row DELETE target
rows versus the kept full-quick baseline.

## Repeat Candidate Ratios

| Scenario | C SQLite median ms | FSQLite median ms | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.005210 | 0.006793 | 1.303839 |
| 100 rows / delete 5 rows | 0.002264 | 0.008245 | 3.641784 |
| 1000 rows / update 100 rows | 0.036298 | 0.030787 | 0.848173 |
| 1000 rows / delete 50 rows | 0.015709 | 0.032711 | 2.082310 |
| 10000 rows / update 1000 rows | 0.357940 | 0.268032 | 0.748818 |
| 10000 rows / delete 500 rows | 0.158667 | 0.299320 | 1.886467 |
