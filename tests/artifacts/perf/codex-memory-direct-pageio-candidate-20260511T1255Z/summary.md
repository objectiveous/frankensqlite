# Memory Direct Page-I/O Candidate - 2026-05-11

Purpose: evaluate a narrow explicit-`:memory:` direct-DML optimization after
the current DML frontier showed fixed transaction/page-state cost and the
logical DELETE buffer family was fenced by prior measurements.

Change under test:

- `crates/fsqlite-core/src/connection.rs`
- `concurrent_page_io_context()` now returns `None` for memory-backed pagers.
  Explicit `BEGIN` still promotes to a concurrent transaction and still
  registers a concurrent session; only direct B-tree DML skips the
  `SharedTxnPageIo` wrapper because the memory pager is private to the
  connection and commit planning still publishes the conservative pager write
  set.

Correctness proof:

- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-memory-direct-pageio-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core page_io_context -- --nocapture`
- Result: passed. The two new tests prove that private `:memory:` explicit
  transactions keep concurrent-session semantics while skipping direct page-I/O
  wrapping across direct INSERT/UPDATE/DELETE, and that file-backed concurrent
  transactions still use shared page-I/O mediation.

Focused UPDATE/DELETE probes:

| Scenario | Baseline F ms | Run 1 F ms | Run 2 F ms |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.006302 | 0.006262 | 0.006191 |
| 100 rows / delete 5 rows | 0.007434 | 0.007364 | 0.007935 |
| 1000 rows / update 100 rows | 0.031378 | 0.028212 | 0.027712 |
| 1000 rows / delete 50 rows | 0.032380 | 0.029756 | 0.029175 |
| 10000 rows / update 1000 rows | 0.294131 | 0.244819 | 0.246932 |
| 10000 rows / delete 500 rows | 0.302406 | 0.262962 | 0.271127 |

The focused profile runs were diagnostic and had `FSQLITE_BENCH_PROFILE_DML=1`.
They show the expected direct fast path remains active (`slow=0`) and the
larger DELETE rows improve in both runs. The 5-row DELETE case is still noisy.

No-profile full quick matrix:

| Metric | Baseline | Candidate run 1 | Candidate run 2 |
| --- | ---: | ---: | ---: |
| FSQLite faster / comparable / C SQLite faster | 79 / 0 / 14 | 79 / 4 / 10 | 83 / 2 / 8 |
| Average F/C | 0.5121668939 | 0.4970694890 | 0.4409629350 |
| Geomean F/C | 0.2732153924 | 0.2738520844 | 0.2600006302 |
| Median F/C | 0.2869799908 | 0.3101066208 | 0.2859674663 |
| p90 F/C | 1.1048789446 | 1.0606407170 | 0.9943873827 |
| p99 F/C | 3.3426258993 | 3.1636675236 | 1.3865603423 |
| Weighted score | 0.3765660231 | 0.3679685474 | 0.3378533753 |

Full-quick DML movement, no profile:

| Scenario | Baseline F/C | Candidate run 1 F/C | Candidate run 2 F/C |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.5051349415 | 1.4396471681 | 1.3865603423 |
| 100 rows / delete 5 rows | 3.3426258993 | 3.1636675236 | 1.3231082846 |
| 1000 rows / delete 50 rows | 2.0612387803 | 1.8763845813 | 0.9172023775 |
| 10000 rows / delete 500 rows | 1.8658629136 | 1.6437117855 | 0.7668442119 |

Decision: keep. Both no-profile full quick runs improve the primary weighted
score and reduce the C SQLite-faster count. The 100-row medium INSERT
regression in candidate run 1 did not repeat in run 2, so it is treated as
measurement noise rather than a new critical red row.
