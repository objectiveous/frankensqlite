# Direct INSERT Single-Pass Record Builder Baseline

- Agent: TanBear
- Timestamp: 2026-05-07T17:57Z
- Git SHA: `e2791375b4e98586f4df771df2bb5ae41138ea0f`
- Benchmark binary: `/data/tmp/frankensqlite-small-insert-target/release-perf/comprehensive-bench`
- Command:
  `env /data/tmp/frankensqlite-small-insert-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/direct-insert-single-pass-record-tanbear-20260507T1757Z/baseline-full.json --no-html`

## Purpose

Capture a same-day full quick baseline for the next unfenced direct INSERT
row-build lever. `crates/fsqlite-core/src/connection.rs` was reserved by
CrimsonGorge during this run, so no source change was made in this artifact.

The candidate held for the reservation window is a single-pass prepared direct
INSERT record builder: carry `(serial_type, payload_len)` beside each
`PreparedDirectInsertRecordValue` during the first value pass, then serialize
from that slot array instead of recomputing layout into a second `SmallVec`.
This mirrors SQLite `OP_MakeRecord`, which stores each field's serial type
during sizing and reuses it while writing the record.

## Baseline Summary

- Total scenarios: `93`
- FrankenSQLite faster: `78`
- Comparable: `5`
- C SQLite faster: `10`
- Average ratio: `0.47969421239591153`
- Geomean ratio: `0.27571799968469085`
- Median ratio: `0.29548253289260484`
- p90 ratio: `1.0546547451926247`
- p99 ratio: `1.6637585529907877`
- Weighted score: `0.3611794741436544`

## Remaining C-Faster Rows

| Ratio | Section | Scenario | C SQLite ms | FrankenSQLite ms |
|---:|---|---|---:|---:|
| 1.6637585529907877 | UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.086081 | 0.143218 |
| 1.4180982337285264 | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.086793 | 0.123081 |
| 1.2094758274258028 | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.067287 | 0.081382 |
| 1.1897254797441366 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.075040 | 0.089277 |
| 1.15857917812291 | INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.148027 | 0.171501 |
| 1.1372307207892733 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.076222 | 0.086682 |
| 1.125233432717882 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 11.683238 | 13.146370 |
| 1.1103475053387692 | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.077265 | 0.085791 |
| 1.0855650693827668 | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | 0.404207 | 0.438793 |
| 1.0546547451926247 | INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.106139 | 0.111940 |
| 1.0282583436506247 | UPDATE/DELETEThroughput | 10000 rows / update 1000 rows | 3.519173 | 3.618619 |
| 1.0243573073114867 | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 0.419135 | 0.429344 |
| 1.000416934911931 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 4 writers x 1000 rows | 20.422852 | 20.431367 |

## Notes

- The DML retained-cursor shell is already rejected in the negative-results
  ledger. Retry DML only with a mutation primitive change, not another cursor
  shell.
- The direct INSERT row-build profile from
  `tests/artifacts/perf/small-insert-ceremony-tanbear-20260507T1741Z/`
  still points at `connection.rs` record construction as the best unfenced
  next lever once the reservation clears.
- The JSON environment reports `benchmark_binary_older_than_git_head: true`
  because `e2791375` was an artifact-only commit after the binary was built;
  no source file changed in that commit.

