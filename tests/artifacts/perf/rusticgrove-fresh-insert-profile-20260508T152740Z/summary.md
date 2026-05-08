# Fresh Insert Profile - 2026-05-08

## Source Basis

- Benchmark source: clean detached worktree `/data/projects/frankensqlite-rusticgrove-clean-profile-20260508T1536Z`
- Git commit: `3872cd79b51b97eb2eeaa555cc87c794ec4ac0bf`
- JSON environment reports `git_dirty=false`
- Main checkout caveat: `crates/fsqlite-core/src/connection.rs` had an uncommitted exact benchmark PRAGMA fast-path patch while this profile was run. That patch was not included in the benchmark binary.

## Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-clean-profile-project-target CARGO_BUILD_JOBS=12 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-rusticgrove-clean-profile-project-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-fresh-insert-profile-20260508T152740Z/insert-profile.json \
  --html /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-fresh-insert-profile-20260508T152740Z/insert-profile.html
```

## Result

- Generated: `2026-05-08 15:42:25 UTC`
- Total scenarios: `25`
- Faster/comparable/slower: `19 / 3 / 3`
- Average ratio: `0.8054164803`
- Geomean ratio: `0.7860424650`
- Weighted observed insert score: `0.8068931946`
- P90 / P99 ratio: `1.0958238691 / 1.1137599553`

Top C-faster rows in this focused run:

| Scenario | C SQLite | FSQLite | F/C |
| --- | ---: | ---: | ---: |
| small_3col 100 rows batched | 0.075211 ms | 0.083767 ms | 1.1138 |
| small_3col 100 rows single txn | 0.073357 ms | 0.081472 ms | 1.1106 |
| large_10col 100 rows single txn | 0.146164 ms | 0.160170 ms | 1.0958 |
| medium_6col 100 rows single txn | 0.101891 ms | 0.106810 ms | 1.0483 |

Large 10K rows did not reproduce as C-faster in this focused run:

| Scenario | C SQLite | FSQLite | F/C |
| --- | ---: | ---: | ---: |
| large_10col 10K single txn | 9.818266 ms | 9.099169 ms | 0.9268 |
| large_10col 10K record size | 9.479391 ms | 9.064715 ms | 0.9563 |

## Insert Profile Counters

All profiled rows used the direct insert path: `direct_insert == fast`, `slow == 0`.

Representative rows:

| Row | setup_us | begin_us | prepare_us | insert_us | commit_us | row_build_ns | btree_insert_ns | commit_roundtrip_ns | page_pool_misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| small_3col 100 batched | 15.4 | 7.9 | 12.7 | 55.9 | 10.8 | 11623 | 3085 | 1383 | 1 |
| small_3col 100 single | 15.5 | 9.5 | 9.8 | 55.9 | 11.1 | 11617 | 3196 | 1523 | 1 |
| large_10col 100 single | 18.5 | 9.7 | 16.7 | 103.7 | 41.3 | 40007 | 5958 | 13506 | 22 |
| large_10col 10K single | 33.1 | 12.8 | 29.3 | 8494.3 | 4031.6 | 3872997 | 640968 | 1953817 | 2006 |
| large_10col 10K record size | 46.6 | 15.3 | 39.9 | 8609.5 | 4376.6 | 3912237 | 671931 | 2204837 | 2006 |

## Decision

No source patch is keepable from this pass.

The current focused evidence points at tiny 100-row fixed costs, not the large-row path as the active C-faster insert frontier. The large-row counters still show meaningful row-build and commit-roundtrip time, but the benchmark rows were faster than C SQLite in this run, so a large-row page-builder patch needs a fresh full-quick reproduction before it clears the keep gate.

The negative-results ledger also blocks the obvious standalone ideas here:

- Exact benchmark PRAGMA parser bypass was already rejected and should not be revived standalone.
- Prepared INSERT row-template and retained leaf-writer fusions were already rejected standalone.
- Page-run/page-builder work should only be retried with narrow admission and full quick plus insert weighted-score proof.

CASS was degraded during this pass: the lexical index was stale, semantic search was unavailable, and `cass index --json` stayed in `preparing` with no discovered agents before being stopped. Current artifacts and the negative-results ledger were therefore treated as the authoritative evidence.
