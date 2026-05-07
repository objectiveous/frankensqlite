# Clean full quick matrix baseline

- Agent: TanBear
- Date: 2026-05-07
- Source: clean detached worktree at `977840591b56b9006b90158c8091529ba2d860a4`
- Worktree: `/data/tmp/frankensqlite-clean-perf-tanbear-20260507T1633Z`
- Build: `release-perf`, `opt-level=3`, LTO
- Mode: `comprehensive-bench --quick --no-html`

## Commands

```bash
git worktree add --detach /data/tmp/frankensqlite-clean-perf-tanbear-20260507T1633Z HEAD
env TMPDIR=/data/tmp/frankensqlite-clean-tanbear-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-clean-perf-target CARGO_BUILD_JOBS=16 rch exec -- cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
/data/tmp/frankensqlite-clean-perf-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/full-quick-clean-tanbear-20260507T1633Z/report.json --no-html
```

## Overall Score

- Total scenarios: `93`
- FrankenSQLite faster: `76`
- Comparable: `3`
- C SQLite faster: `14`
- Average ratio: `0.488036`
- Geomean ratio: `0.277289`
- Median ratio: `0.298061`
- P90 ratio: `1.085722`
- P99 ratio: `1.545819`
- Primary weighted score: `0.360871`

## Category Read

- `write_single`: geomean `1.041813`, 4 C-faster rows, 1 comparable row, 4 FrankenSQLite-faster rows.
- `write_bulk`: geomean `0.896052`, 10 C-faster rows, 12 FrankenSQLite-faster rows.
- `concurrent_writers`: geomean `0.739372`, 2 C-faster rows, 1 FrankenSQLite-faster row.
- `read_single`, `read_aggregate`, and `mixed`: no C-faster rows in this run.

## Remaining C-Faster Rows

| Ratio | Category | Section | Scenario | C median ms | F median ms |
| ---: | --- | --- | --- | ---: | ---: |
| 1.545819 | write_single | UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.082084 | 0.126887 |
| 1.479717 | write_single | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.079820 | 0.118111 |
| 1.209441 | write_bulk | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.072856 | 0.088115 |
| 1.200787 | write_bulk | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B: includes long text fields) | 9.321958 | 11.193683 |
| 1.181658 | write_bulk | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.073297 | 0.086612 |
| 1.169459 | write_bulk | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.073959 | 0.086492 |
| 1.158496 | write_bulk | INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.148729 | 0.172302 |
| 1.150542 | write_bulk | INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.103353 | 0.118912 |
| 1.107821 | write_bulk | INSERTThroughput - Single Transaction - medium_6col | 10000 rows | 5.711056 | 6.326829 |
| 1.085722 | write_single | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | 0.402524 | 0.437029 |
| 1.075998 | concurrent_writers | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 12.521179 | 13.472761 |
| 1.068180 | write_single | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 0.366176 | 0.391142 |
| 1.064453 | write_bulk | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.066855 | 0.071164 |
| 1.050494 | write_bulk | INSERTThroughput - Single Transaction - medium_6col | 1000 rows | 0.542266 | 0.569647 |
| 1.041261 | write_bulk | INSERTThroughput - Single Transaction - large_10col | 1000 rows | 0.899635 | 0.936755 |
| 1.015290 | concurrent_writers | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 4 writers x 1000 rows | 19.411293 | 19.708088 |

## Interpretation

This supersedes the inherited `full-quick-current-tanbear-20260507T1613Z`
bundle as the clean baseline. That earlier run recorded `Git dirty: yes`; the
dirty file was `crates/fsqlite-core/src/connection.rs`, containing the rejected
prepared concat-specialization experiment, so it should not be used as a clean
`main` benchmark gate.

The clean baseline says the next highest-impact surfaces are:

1. `write_single` small UPDATE/DELETE. This remains the worst geomean category,
   but the existing UPDATE/DELETE profile shows full-row cost is mostly
   setup/prepare for the tiny benchmark rows while isolated direct DML still has
   a real per-row gap. Do not retry one-row leaf hints or fixed-width payload
   tweaks already fenced in the negative ledger.
2. `write_bulk` insert rows. The 100-row fixed-cost rows and the 10K large-row
   payload row are both C-faster, so the next candidate should be selected by
   profiling one exact row first rather than broadening the right-edge page-run
   admission again.
3. Low-writer-count concurrent rows. The 8-writer row is already faster, so any
   concurrency change must keep the high-writer advantage and prove the 2/4
   writer rows move in the full matrix.
