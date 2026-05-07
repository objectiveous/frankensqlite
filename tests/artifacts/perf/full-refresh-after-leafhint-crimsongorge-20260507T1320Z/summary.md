# Full Matrix Check for Direct-DML Leaf Hint Candidate - 2026-05-07

Agent: CrimsonGorge

Measured commit: `6e13684fd6a95ae9ab55613e5942dbc76b684348`
(`perf(direct-dml): cache hinted leaf for repeated fixed-width UPDATEs`).
This commit was still `HEAD` when the clean worktree was created. A peer
focused update/delete artifact rejected the candidate while this full-matrix
run was in progress, so this artifact is supplemental reject/diagnostic
evidence, not a keep recommendation.

Measurement worktree:
`/data/projects/frankensqlite-clean-head-crimsongorge-20260507T1320Z`.

Build command:

```bash
rch exec -- env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-head-leafhint-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Benchmark command:

```bash
/data/tmp/frankensqlite-current-head-leafhint-target/release-perf/comprehensive-bench \
  --quick \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/full-refresh-after-leafhint-crimsongorge-20260507T1320Z/report-full.json \
  --no-html
```

## Result

- Total scenarios: 93.
- FrankenSQLite faster: 78.
- Comparable: 3.
- C SQLite faster: 12.
- Primary metric: `per_category_weighted.score`.
- Primary weighted score: `0.36325874598538`.
- Geomean ratio: `0.28174179231182767`.
- Median ratio: `0.2942716648572706`.
- `p90_ratio`: `1.1407087103183295`.
- `p99_ratio`: `1.4530921554586331`.

Compared with `tests/artifacts/perf/full-refresh-crimsongorge-20260507T1246Z/report-full.json`,
the leaf-hint candidate did improve the previous worst UPDATE row:
`100 rows / update 10 rows` moved from ratio `1.5415` to `0.9494`. The full
matrix still does not justify keeping the candidate by itself: primary score
was effectively flat/slightly worse (`0.36267 -> 0.36326`), geomean worsened
(`0.27514 -> 0.28174`), `10000 rows / delete 500 rows` crossed back to
C-faster, and the biggest remaining gap worsened on batched INSERT.

## Remaining C-Faster Rows

| Ratio | Section | Scenario | C SQLite ms | FrankenSQLite ms | Category |
| ---: | --- | --- | ---: | ---: | --- |
| 1.4531 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 10000 rows / batched (1000/txn) | 3.157393 | 4.587983 | write_bulk |
| 1.4367 | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.082704 | 0.118823 | write_single |
| 1.3678 | INSERTThroughput - Single Transaction - medium_6col | 1000 rows | 0.545632 | 0.746298 | write_bulk |
| 1.2353 | INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.158597 | 0.195917 | write_bulk |
| 1.2091 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.075492 | 0.091281 | write_bulk |
| 1.1788 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.075552 | 0.089057 | write_bulk |
| 1.1781 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 12.111846 | 14.269277 | concurrent_writers |
| 1.1728 | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B) | 9.706080 | 11.383231 | write_bulk |
| 1.1609 | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.076944 | 0.089327 | write_bulk |
| 1.1407 | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | 0.388057 | 0.442660 | write_single |
| 1.0952 | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 0.366867 | 0.401782 | write_single |
| 1.0516 | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.070312 | 0.073939 | write_bulk |
| 1.0023 | UPDATE/DELETEThroughput | 10000 rows / delete 500 rows | 3.309569 | 3.317313 | write_single |

## Diagnosis

After rejecting the leaf-hint candidate as a standalone direct-DML optimization,
the highest-EV target is again the `small_3col` `10000 rows / batched
(1000/txn)` row. Multiple ledger entries rule out row-at-a-time non-empty
page-run replay, direct writer callbacks, standalone record-layout reuse, and
local pointer/copy micro-tweaks. A viable next attempt needs a true
non-empty-root page/run builder, or a different transaction-boundary strategy
that removes repeated per-batch setup without shifting work into commit.

For UPDATE/DELETE, the full matrix agrees with the focused reject: the tiny
UPDATE row moved, but delete rows and larger update rows do not improve enough
to offset risk. Do not keep a connection-level last-leaf hint without a broader
retained-cursor direct-DML kernel that also fixes delete-path overhead.
