# Current Full-Quick Refresh

- Run timestamp from benchmark output: 2026-05-16 00:41:24 UTC.
- Artifact directory note: the directory name has the local-date prefix from
  the shell session; the benchmark's own UTC timestamp above is the canonical
  run time.
- Repository: `/data/projects/frankensqlite`
- Git `HEAD`: `06a37f61e0ad97ffa95449f2f97a27ea080c821c`
- Worktree: dirty; staged perf evidence was present and
  `crates/fsqlite-core/src/connection.rs` still had the unstaged ALTER TABLE
  rename correctness changes.
- Command:
  `rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --json-out tests/artifacts/perf/codex-current-fullquick-refresh-20260515T003425Z/full-quick.json --no-html`
- Local artifacts: `run.log`. RCH reported that the JSON report was written on
  the remote worker, but `full-quick.json` was not present locally after
  artifact retrieval.

## Matrix Summary

The refreshed quick matrix reported:

- Total scenarios: `93`
- FrankenSQLite faster: `80`
- Comparable: `1`
- C SQLite faster: `12`
- Average F/C time ratio: `0.48x`

The previously red concurrent-writer rows are green in this run:

| Scenario | C SQLite | FrankenSQLite | Ratio | C CV | F CV |
|---|---:|---:|---:|---:|---:|
| 2 writers x 1000 rows | 97.69 ms | 23.21 ms | 4.21x faster | 12.1% | 14.7% |
| 4 writers x 1000 rows | 115.67 ms | 35.31 ms | 3.28x faster | 22.3% | 13.8% |
| 8 writers x 1000 rows | 244.43 ms | 73.88 ms | 3.31x faster | 6.2% | 18.4% |

## Current C-SQLite-Faster Rows

The log prints 13 `x slower` rows; the report summary classifies 12 as
C-SQLite-faster and one as comparable. The slower rows are:

| Section | Scenario | C SQLite | FrankenSQLite | Ratio | C CV | F CV |
|---|---|---:|---:|---:|---:|---:|
| INSERT single txn tiny_1col | 100 rows | 105.1 us | 117.5 us | 1.12x slower | 35.4% | 52.1% |
| INSERT single txn small_3col | 100 rows | 106.7 us | 140.7 us | 1.32x slower | 27.2% | 21.0% |
| INSERT single txn medium_6col | 100 rows | 149.0 us | 163.1 us | 1.09x slower | 25.3% | 11.3% |
| INSERT single txn large_10col | 100 rows | 218.9 us | 241.6 us | 1.10x slower | 12.4% | 5.6% |
| INSERT single txn large_10col | 10000 rows | 14.38 ms | 15.67 ms | 1.09x slower | 0.5% | 8.2% |
| INSERT transaction strategy small_3col | 100 rows / autocommit | 188.6 us | 229.0 us | 1.21x slower | 10.5% | 20.4% |
| INSERT transaction strategy small_3col | 100 rows / batched (100/txn) | 110.2 us | 128.6 us | 1.17x slower | 15.4% | 20.9% |
| INSERT transaction strategy small_3col | 100 rows / single txn | 131.8 us | 142.2 us | 1.08x slower | 22.3% | 16.5% |
| INSERT record-size comparison | large_10col / 10000 rows | 14.55 ms | 15.03 ms | 1.03x slower | 3.7% | 1.9% |
| UPDATE/DELETE | 100 rows / update 10 rows | 6.2 us | 8.5 us | 1.39x slower | 22.9% | 3.9% |
| UPDATE/DELETE | 100 rows / delete 5 rows | 3.3 us | 9.6 us | 2.88x slower | 2.8% | 3.6% |
| UPDATE/DELETE | 1000 rows / delete 50 rows | 24.4 us | 44.3 us | 1.81x slower | 19.5% | 6.3% |
| UPDATE/DELETE | 10000 rows / delete 500 rows | 260.8 us | 418.7 us | 1.61x slower | 18.4% | 3.3% |

## Interpretation

This refresh supersedes the older
`tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/remaining-fullquick-gap-triage.md`
for current red-row selection. The concurrent-writer rows are no longer current
source targets from this matrix.

The main remaining source lever is still the transaction-local DML mutation
operator for DELETE and the 100-row UPDATE fixed-cost tail. The INSERT rows are
mostly 100-row fixed-cost tails or low-ratio large-row construction tails; they
do not clear the implementation gate without a sharper profile.
