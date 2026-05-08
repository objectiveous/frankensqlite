# Full Quick Frontier Refresh - 2026-05-08

## Source Basis

- Benchmark source: clean detached worktree `/data/projects/frankensqlite-rusticgrove-full-refresh-20260508T1547Z`
- Git commit: `8c8fdef3dac8f30d8bc5a07f6338e6d0bf818aab`
- Full quick JSON environment reports `git_dirty=false`
- Build: `release-perf`
- Main checkout caveat: `crates/fsqlite-core/src/connection.rs` had an unstaged exact benchmark PRAGMA fast-path patch while this artifact was produced. The patch was not included in the benchmark binary.

## Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-full-refresh-target CARGO_BUILD_JOBS=12 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

/data/tmp/frankensqlite-rusticgrove-full-refresh-target/release-perf/comprehensive-bench \
  --quick \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-full-quick-refresh-20260508T1547Z/full-quick.json \
  --html /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-full-quick-refresh-20260508T1547Z/full-quick.html

FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-rusticgrove-full-refresh-target/release-perf/comprehensive-bench \
  --quick --filter dml \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-full-quick-refresh-20260508T1547Z/dml-profile.json \
  --html /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-full-quick-refresh-20260508T1547Z/dml-profile.html

FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-rusticgrove-full-refresh-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-full-quick-refresh-20260508T1547Z/insert-rerun-profile.json \
  --html /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-full-quick-refresh-20260508T1547Z/insert-rerun-profile.html
```

## Full Quick Result

- Generated: `2026-05-08 15:52:23 UTC`
- Total scenarios: `93`
- Faster/comparable/slower: `81 / 3 / 9`
- Average ratio: `0.4500656955`
- Geomean ratio: `0.2662743507`
- Weighted score: `0.3430698050`
- P90 / P99 ratio: `1.0385673209 / 1.3090301761`

Compared with `rusticgrove-full-quick-current-20260508T1510Z`, the weighted score is effectively unchanged (`0.3431` vs `0.3412`). This refresh is a frontier snapshot, not a source improvement.

Top slow or near-slow rows:

| Scenario | Category | C SQLite | FSQLite | F/C | Notes |
| --- | --- | ---: | ---: | ---: | --- |
| 100 rows / update 10 rows | write_single | 0.088315 ms | 0.115607 ms | 1.3090 | stable top full-matrix gap |
| 100 rows / delete 5 rows | write_single | 0.084207 ms | 0.105608 ms | 1.2541 | stable top full-matrix gap |
| medium_6col 1000 rows insert | write_bulk | 0.563464 ms | 0.688559 ms | 1.2220 | F CV 17.7%, needs repeat |
| large_10col 10K single txn insert | write_bulk | 9.525558 ms | 10.842302 ms | 1.1382 | F CV 11.9%, profile rerun only mild |
| tiny_1col 100 rows insert | write_bulk | 0.068519 ms | 0.076804 ms | 1.1209 | fixed-cost tail |
| small_3col 100 rows batched insert | write_bulk | 0.078146 ms | 0.085760 ms | 1.0974 | fixed-cost tail |
| large_10col 100 rows insert | write_bulk | 0.150502 ms | 0.164077 ms | 1.0902 | fixed-cost tail |
| small_3col 100 rows single txn insert | write_bulk | 0.078547 ms | 0.083907 ms | 1.0682 | fixed-cost tail |
| 2 writers x 1000 rows | concurrent_writers | 12.034804 ms | 12.766022 ms | 1.0608 | low-thread concurrent tail |

## DML Profile

Focused DML run:

- Generated: `2026-05-08 15:52:57 UTC`
- Faster/comparable/slower: `1 / 4 / 1`
- Average/geomean: `1.0485220850 / 1.0385197368`
- P90/P99: `1.3898699979 / 1.3898699979`

Rows:

| Scenario | C SQLite | FSQLite | F/C | Notes |
| --- | ---: | ---: | ---: | --- |
| 100 rows / update 10 rows | 0.1126 ms | 0.1166 ms | 1.03 | C CV 46.5%, not stable in focused run |
| 100 rows / delete 5 rows | 0.0807 ms | 0.1122 ms | 1.39 | focused run confirms delete tail |
| 1000 rows / update 100 rows | 0.4149 ms | 0.4145 ms | 1.00 | comparable |
| 1000 rows / delete 50 rows | 0.3751 ms | 0.3561 ms | 0.95 | faster |
| 10000 rows / update 1000 rows | 3.63 ms | 3.50 ms | 0.96 | comparable/faster |
| 10000 rows / delete 500 rows | 3.36 ms | 3.20 ms | 0.95 | comparable/faster |

100-row phase counters:

| Row | setup_us | begin_us | prepare_us | mutate_us | commit_us | direct ops | page misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| update 10/100 | 54.5 | 6.9 | 13.0 | 12.9 | 5.8 | 10 | 0 |
| delete 5/100 | 55.6 | 5.0 | 11.9 | 8.7 | 5.2 | 5 | 0 |

The direct mutation path is not the main cost. The measured DML gap is mostly benchmark setup/prepopulation plus fixed prepare/begin/commit work. The direct DML path itself is already fast.

## Insert Rerun

Focused insert rerun:

- Generated: `2026-05-08 15:53:21 UTC`
- Faster/comparable/slower: `18 / 2 / 5`
- Average/geomean: `0.8005629254 / 0.7729981734`
- Observed insert weighted score: `0.7849050074`

Top insert slow rows:

| Scenario | C SQLite | FSQLite | F/C | Notes |
| --- | ---: | ---: | ---: | --- |
| small_3col 100 rows | 0.076884 ms | 0.088164 ms | 1.1467 | fixed-cost tail |
| small_3col 100 rows batched | 0.075280 ms | 0.085269 ms | 1.1327 | fixed-cost tail |
| large_10col 100 rows | 0.145311 ms | 0.160580 ms | 1.1051 | fixed-cost tail |
| small_3col 100 rows single txn | 0.075501 ms | 0.082534 ms | 1.0932 | fixed-cost tail |
| large_10col 10K record-size | 9.577504 ms | 10.369191 ms | 1.0827 | mild and noisy; previous focused profile was faster |

Representative insert counters:

| Row | setup_us | begin_us | prepare_us | insert_us | commit_us | row_build_ns | btree_insert_ns | commit_roundtrip_ns | page misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| small_3col 100 | 15.1 | 8.7 | 9.7 | 55.7 | 10.4 | 11344 | 3244 | 1462 | 1 |
| large_10col 100 | 18.7 | 10.1 | 17.2 | 86.7 | 40.1 | 39860 | 5581 | 13305 | 22 |
| large_10col 10K single | 30.9 | 12.9 | 27.5 | 8582.2 | 4073.3 | 3966647 | 622787 | 2021465 | 2006 |
| large_10col 10K record-size | 45.1 | 16.0 | 36.5 | 8834.8 | 4451.2 | 4139238 | 745178 | 2143142 | 2006 |

## Negative-Ledger Fence

The obvious nearby standalone ideas remain blocked by `docs/progress/perf-negative-results.md`:

- exact benchmark PRAGMA fast path
- direct INSERT concat record-body encoder
- prepared INSERT row-template executor
- retained leaf-writer serialization fusion
- broad depth-2 page builder admission
- direct DML retained seek/cursor hints
- fixed-width REAL update and leaf-local DML patches
- private-memory direct UPDATE/DELETE page-I/O bypass

## Decision

No source patch was applied.

The current repeatable frontier is shared fixed setup/tiny-row cost, with 100-row DML and 100-row INSERT tails moving together. The large-row page-builder idea remains plausible, but the evidence is inconsistent across focused insert runs; it needs a repeated full-plus-focused reproduction before source work.

Next keep gate:

1. Candidate must improve both 100-row INSERT and 100-row DML, or must repeatedly reproduce and improve the large_10col 10K rows.
2. Candidate must not revive any standalone negative-ledger reject.
3. Candidate must win focused profile first, then full quick weighted score and slow-row count.
4. CASS remains degraded for recent May 8 sessions, so current artifacts and the negative ledger are the source of truth until indexing is healthy.
