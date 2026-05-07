# Clean current focused INSERT profile

Date: 2026-05-07 18:53Z
Agent: TanBear

## Scope

Focused clean INSERT pass from the same detached worktree used for the current
full quick baseline:

`/data/tmp/frankensqlite-clean-current-tanbear-20260507T1846Z`

Source: `13d3d03b2eb064f7be16ea35a2492aebb42ff208`

The worktree was clean (`git_dirty=false`) and the release-perf benchmark
binary was not older than the git head.

## Commands

Non-profiled focused INSERT pass:

```text
/data/tmp/frankensqlite-clean-current-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/insert-profile-clean-current-tanbear-20260507T1853Z/insert-profile-clean-current.json \
  --no-html
```

Profiled focused INSERT pass:

```text
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-clean-current-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/insert-profile-clean-current-tanbear-20260507T1853Z/insert-profile-clean-current-profiled.json \
  --no-html
```

Logs:

- `stdout/insert-profile-clean-current.err`
- `stdout/insert-profile-clean-current-profiled.err`

## Profiled summary

The profiling flag perturbs timings, so use this run primarily for attribution.

| Metric | Value |
| --- | ---: |
| Total scenarios | 25 |
| Franken faster | 15 |
| Comparable | 3 |
| C SQLite faster | 7 |
| Average ratio | 0.9297404143407193 |
| Geomean ratio | 0.8974933738758618 |
| Median ratio | 0.8626715210633876 |
| p90 ratio | 1.262157308415411 |
| p99 ratio | 1.5085532513276994 |
| Weighted score | 0.8716115586423964 |

Strict ratio-over-1 rows in the profiled focused run:

| Scenario | Ratio | C SQLite | FrankenSQLite |
| --- | ---: | ---: | ---: |
| 100 rows / small_3col insert | 1.5085532513276994 | 0.073247 ms | 0.110497 ms |
| 100 rows / large_10col insert | 1.3340130448749283 | 0.143198 ms | 0.191028 ms |
| 100 rows / batched (100/txn) | 1.262157308415411 | 0.071274 ms | 0.089959 ms |
| 100 rows / tiny_1col insert | 1.2469149871389966 | 0.061426 ms | 0.076593 ms |
| 100 rows / single txn | 1.2422392913985192 | 0.072255 ms | 0.089758 ms |
| 1000 rows / medium_6col insert | 1.2189395591691088 | 0.609803 ms | 0.743313 ms |
| 100 rows / medium_6col insert | 1.0635156527638678 | 0.123812 ms | 0.131676 ms |
| 10000 rows / large_10col insert | 1.0235799398796532 | 9.617709 ms | 9.844494 ms |

## Hot-path attribution

Selected `insert_profile` counters from
`stdout/insert-profile-clean-current-profiled.err`:

| Label | Rows | execute_body_ns | row_build_ns | btree_insert_ns | memdb_apply_ns | schema_validation_ns | change_tracking_ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| single_txn_tiny_1col_100 | 100 | 98732 | 5139 | 4550 | 3066 | 4560 | 3063 |
| single_txn_small_3col_100 | 100 | 88717 | 18404 | 4317 | 2484 | 3837 | 2470 |
| single_txn_medium_6col_100 | 100 | 67664 | 22008 | 3444 | 2356 | 3226 | 2465 |
| single_txn_large_10col_100 | 100 | 115997 | 44979 | 5939 | 2385 | 3459 | 2425 |
| single_txn_small_3col_1000 | 1000 | 375478 | 134439 | 29868 | 24155 | 31302 | 23657 |
| single_txn_medium_6col_1000 | 1000 | 491305 | 199790 | 32741 | 23927 | 31256 | 23916 |
| single_txn_large_10col_1000 | 1000 | 889044 | 411891 | 47891 | 24013 | 31524 | 24110 |
| single_txn_small_3col_10000 | 10000 | 4371198 | 1488774 | 734988 | 242222 | 321546 | 239048 |
| single_txn_medium_6col_10000 | 10000 | 4845678 | 2063694 | 361047 | 240479 | 329315 | 238295 |
| single_txn_large_10col_10000 | 10000 | 11173863 | 4404284 | 794309 | 260489 | 331414 | 239144 |
| record_size_large_10col_10000 | 10000 | 10522806 | 4289436 | 698893 | 241691 | 317205 | 238630 |

## Readout

The clean profile keeps pointing at the same next lever:

- For 100-row small/medium/large inserts, `row_build_ns` is already larger than
  B-tree insertion, MemDatabase apply, schema validation, and change tracking.
- For larger payload rows, `row_build_ns` remains the largest named executor
  component and scales with column count/payload size.
- `serialize_ns=0` in these counters means the direct prepared INSERT path is
  doing record construction under `row_build_ns`; the next candidate should
  reduce layout/value ceremony inside the direct record builder rather than
  retry the fenced generic record serializer paths.

The patch target remains `crates/fsqlite-core/src/connection.rs`, specifically
the direct prepared INSERT record-builder path. That file is currently
peer-reserved, so this artifact is the clean proof pack for the next source
owner rather than a source change.
