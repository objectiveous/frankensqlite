# Clean current full quick baseline

Date: 2026-05-07 18:46Z
Agent: TanBear

## Scope

Refreshed the full quick benchmark matrix from a clean detached worktree at
`13d3d03b2eb064f7be16ea35a2492aebb42ff208`. This avoids the main worktree's
peer-owned dirty edits in `crates/fsqlite-core/src/connection.rs`,
`crates/fsqlite-btree/src/cursor.rs`, and
`docs/progress/perf-negative-results.md`.

Worktree:
`/data/tmp/frankensqlite-clean-current-tanbear-20260507T1846Z`

Build:

```text
env TMPDIR=/data/tmp/frankensqlite-clean-current-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-clean-current-target \
  CARGO_BUILD_JOBS=12 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Run:

```text
/data/tmp/frankensqlite-clean-current-target/release-perf/comprehensive-bench \
  --quick \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/full-quick-clean-current-tanbear-20260507T1846Z/full-quick-clean-current.json \
  --no-html
```

Logs:

- `stdout/build-bench.log`
- `stdout/full-quick-clean-current.err`

## Benchmark metadata

- Source: `13d3d03b2eb064f7be16ea35a2492aebb42ff208`
- Git dirty: `false`
- Benchmark binary older than git head: `false`
- Profile: `release-perf`

## Summary

| Metric | Value |
| --- | ---: |
| Total scenarios | 93 |
| Franken faster | 78 |
| Comparable | 4 |
| C SQLite faster | 11 |
| Average ratio | 0.48746218719442136 |
| Geomean ratio | 0.2748904916027281 |
| Median ratio | 0.2996830266793579 |
| p90 ratio | 1.0752947769502312 |
| p99 ratio | 1.8825423374451182 |
| Weighted score | 0.3583314172356216 |

Per-category:

| Category | Rows | Avg ratio | Geomean | p90 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: |
| concurrent_writers | 3 | 0.8553644208686796 | 0.7701934707710407 | 1.1436001691635034 | 1.1436001691635034 |
| mixed | 1 | 0.19161506558396985 | 0.19161506558396985 | 0.19161506558396985 | 0.19161506558396985 |
| read_aggregate | 25 | 0.21652790779725448 | 0.07318406874402264 | 0.49355864393751486 | 0.7037756606252534 |
| read_single | 33 | 0.233964352778461 | 0.22258781553480708 | 0.3485808006426563 | 0.46537575618035054 |
| write_bulk | 22 | 0.9213335910960574 | 0.884290231382273 | 1.1605262899696824 | 1.8825423374451182 |
| write_single | 9 | 1.0192127489063703 | 0.9991251093746504 | 1.5749222579455016 | 1.5749222579455016 |

## Remaining ratio-over-1 rows

Strict `FrankenSQLite / C SQLite > 1.0` rows:

| Scenario | Ratio | C SQLite | FrankenSQLite |
| --- | ---: | ---: | ---: |
| 100 rows / single txn | 1.8825423374451182 | 0.071745 ms | 0.135063 ms |
| 100 rows / delete 5 rows | 1.5749222579455016 | 0.075892 ms | 0.119524 ms |
| 100 rows / small_3col insert | 1.1794577300135363 | 0.073137 ms | 0.086262 ms |
| 100 rows / medium_6col insert | 1.1605262899696824 | 0.101921 ms | 0.118282 ms |
| 100 rows / tiny_1col insert | 1.1593495238503162 | 0.070041 ms | 0.081202 ms |
| 2 writers x 1000 rows | 1.1436001691635034 | 12.298161 ms | 14.064179 ms |
| 10000 rows / medium_6col insert | 1.1412830408650685 | 5.400358 ms | 6.163337 ms |
| large_10col 10K record-size row | 1.1060500857276458 | 9.6235 ms | 10.644073 ms |
| 100 rows / update 10 rows | 1.077637971845023 | 0.120405 ms | 0.129753 ms |
| 1000 rows / update 100 rows | 1.0752947769502312 | 0.399709 ms | 0.429805 ms |
| 100 rows / large_10col insert | 1.0734264415643189 | 0.159098 ms | 0.17078 ms |
| 4 writers x 1000 rows | 1.037381224615463 | 19.227995 ms | 19.946761 ms |
| 1000 rows / delete 50 rows | 1.0269847487525314 | 0.390591 ms | 0.401131 ms |

## Readout

The remaining clean gaps are now narrow:

- Small-row insert ceremony and row building still dominate the write-bulk tail.
  This points back to the direct INSERT row builder in
  `crates/fsqlite-core/src/connection.rs`, which is currently peer-reserved.
- Small UPDATE/DELETE rows remain the write-single tail. The ledger now fences
  retained direct-DML cursor shells, lazy fallback compilation, QF consultation,
  microbatch schema-proof carries, scratch-reset trims, fixed-payload patches,
  and leaf hints as standalone retries.
- Low-writer concurrent rows remain slightly behind, while 8-writer MVCC is
  still much faster than C SQLite WAL. Prior profiling attributes the low-writer
  gap mostly to direct INSERT row construction and B-tree page/layout work, not
  the benchmark runtime wrapper.

Next high-EV source lane, once reservations clear, is still a single-pass direct
INSERT record slot/layout builder in `connection.rs`, followed by a same-window
focused insert matrix and then this full quick matrix.
