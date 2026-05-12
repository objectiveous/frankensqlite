# Current DML Refresh After INSERT Frontier

- Date: 2026-05-12 16:15 UTC
- Commit: `ececff307d54c95dc5ffca6b3a1e14478e720b42`
- Focused command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-dml-probe-ececff30 CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html --json-out tests/artifacts/perf/codex-current-dml-refresh-after-insert-frontier-20260512T1615Z/dml-current.json`
- Repeat command:
  same command shape, writing `dml-repeat.json`.
- Focused probes:
  `env FSQLITE_HOT_PROFILE=1 /data/tmp/frankensqlite-target-dml-probe-ececff30/release-perf/perf-update-delete 10000 200 delete compare <mode>`.
- Validity: both JSON runs report `git_dirty=false`,
  `benchmark_binary_older_than_git_head=false`, and `build_profile=release-perf`.

## Summary

The first focused DML run covered 6 scenarios:

| Metric | Value |
| --- | ---: |
| Franken faster / comparable / C faster | `1 / 0 / 5` |
| Average F/C | `1.6979126433835754` |
| Geomean F/C | `1.546796183695123` |
| Focused weighted score | `1.546796183695123` |

The repeat run covered the same 6 scenarios:

| Metric | Value |
| --- | ---: |
| Franken faster / comparable / C faster | `2 / 0 / 4` |
| Average F/C | `1.4570830745315133` |
| Geomean F/C | `1.3229800457307936` |
| Focused weighted score | `1.3229800457307936` |

## Rows

| Scenario | Current F/C | Repeat F/C | Current C median | Current F median | Repeat C median | Repeat F median |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `100 rows / update 10 rows` | `1.4643536121673004` | `1.4079566854990584` | `0.004208ms` | `0.006162ms` | `0.004248ms` | `0.005981ms` |
| `100 rows / delete 5 rows` | `3.101133391455972` | `2.4587332053742803` | `0.002294ms` | `0.007114ms` | `0.003647ms` | `0.008967ms` |
| `1000 rows / update 100 rows` | `1.3502413482778368` | `0.7715507664963341` | `0.036669ms` | `0.049512ms` | `0.036008ms` | `0.027782ms` |
| `1000 rows / delete 50 rows` | `1.8212064676616915` | `1.7957473420888055` | `0.016080ms` | `0.029285ms` | `0.015990ms` | `0.028714ms` |
| `10000 rows / update 1000 rows` | `0.701052256406401` | `0.6912339143021434` | `0.359133ms` | `0.251771ms` | `0.353031ms` | `0.244027ms` |
| `10000 rows / delete 500 rows` | `1.7494887843322497` | `1.6172765334284587` | `0.160891ms` | `0.281477ms` | `0.159789ms` | `0.258423ms` |

The `1000 rows / update 100 rows` row flipped from red to green on repeat,
and the first run's FrankenSQLite CV was `74.51%`. Treat that as noise. The
large update row is green in both runs with low enough CV to treat as stable.
The stable red rows are still DELETE, especially `50/1000` and `500/10000`.

## Delete Mode Probes

The focused `perf-update-delete` probes used the same current binary and 200
iterations over the 10000-row, 500-delete workload:

| Mode | Total F/C | Delete F/C | F delete | C delete | F per row | C per row |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `standard` | `0.89x` | `1.50x` | `49ms` | `33ms` | `497ns` | `331ns` |
| `isolated` | `1.04x` | `1.22x` | `35ms` | `29ms` | `357ns` | `293ns` |
| `rollback-isolated` | `7.20x` | `1.09x` | `32ms` | `30ms` | `328ns` | `301ns` |
| `sparse-isolated` | `0.95x` | `2.26x` | `104ms` | `46ms` | `1046ns` | `463ns` |

The rollback-isolated inner delete kernel is close to C SQLite. Standard and
isolated still show a smaller delete gap, while sparse-isolated remains the
clearest red shape.

## Profile Notes

Representative repeat-run `500/10000` DELETE counters:

| Counter | Value |
| --- | ---: |
| `mutate_us` | `244.1` |
| `commit_us` | `39.7` |
| `direct_delete` | `500` |
| `fast / slow` | `500 / 0` |
| `delete_seek_ns` | `33229` |
| `delete_physical_ns` | `11332` |
| `delete_leaf_start` | `64 / 67` |
| `delete_leaf_active` | `433 / 496` |
| `delete_leaf_miss` | `63` |
| `delete_leaf_miss_out_of_leaf` | `60` |
| `delete_leaf_miss_last_cell` | `3` |
| `delete_leaf_flush` | `64 / 64` |
| `delete_leaf_flush_ns` | `51767` |
| `delete_leaf_materialize` | `64 / 38505` |
| `delete_leaf_write` | `64 / 7587` |
| `bg_checks / bg_ns` | `504 / 13877` |

All DELETE rows stayed on the direct path (`slow=0`). The repeated misses are
shape-boundary effects from the every-20th-row sparse delete pattern, not a
fallback to VDBE or a newly exposed slow path.

## Decision

No source patch from this refresh. The result reinforces the existing ledger:
standalone transaction-envelope trimming, retained same-leaf delete-run tweaks,
sparse cursor wrappers, dense-rowid queues, tombstone queues, direct-flush
wrappers, and page-1/commit shortcuts are already fenced by measured rejects.

The next source attempt should be the broader transaction-local DML mutation
operator only if it can prove focused DELETE wins, preserve reads/rollback/
savepoint behavior, and keep the fullquick primary score neutral or better.
