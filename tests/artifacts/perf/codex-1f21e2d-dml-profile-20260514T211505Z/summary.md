# DELETE DML Profile - 2026-05-14

Run id: `codex-1f21e2d-dml-profile-20260514T211505Z`
Head: `1f21e2d3c29c065753e3f02a5fca6a982f97e35c`

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-1f21e2d-dml-profile-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 20 delete compare standard
```

Primary log: `perf-update-delete-10000x20-delete-compare-standard.log`
Environment fingerprint: `fingerprint.txt`

The remote benchmark command reported `exit=0`. The local `rch` wrapper was
stopped after benchmark stdout was captured and after the project artifact
retrieval phase, while it was retrieving the worker `.rch-target` directory.

## Headline Result

Workload: `perf-update-delete`, `rows=10000`, `iters=20`, `which=delete`,
`engine=compare`, `mode=standard`.

| Engine | Total | Populate | Delete | Per-row DELETE |
| --- | ---: | ---: | ---: | ---: |
| FrankenSQLite | 205 ms | 147 ms | 21 ms | 2170 ns |
| C SQLite | 177 ms | 159 ms | 9 ms | 989 ns |
| Ratio | 1.16x | 0.93x | 2.19x | 2.19x |

Populate remains favorable for FrankenSQLite, but the DELETE phase is still the
active C SQLite-faster gap.

## Averaged FrankenSQLite Hotspots

These are means across the 20 profiled FrankenSQLite iterations in the log.

| Counter | Mean |
| --- | ---: |
| `elapsed_us` | 1085.5 us |
| `delete_leaf_active_ns` | 317.2 us |
| `delete_leaf_flush_ns` | 165.3 us |
| `delete_leaf_materialize` | 64 calls |
| `delete_leaf_materialize_ns` | 124.7 us |
| `execute_body_ns` | 144.4 us |
| `delete_leaf_search` | 560 calls |
| `delete_leaf_search_ns` | 92.2 us |
| `delete_seek_ns` | 139.1 us |
| `commit_roundtrip_ns` | 73.8 us |
| `delete_leaf_dupcheck` | 500 calls |
| `delete_leaf_dupcheck_ns` | 24.6 us |
| `delete_leaf_compact` | 497 calls |
| `delete_leaf_compact_ns` | 37.1 us |
| `delete_leaf_cellparse` | 497 calls |
| `delete_leaf_cellparse_ns` | 26.2 us |
| `bg_ns` | 20.2 us |

Shape counters remained stable: each iteration did 500 direct deletes, 64
leaf-run flushes, 433 active leaf deletes across 496 active attempts, and 63
active-run misses. Misses were again mostly
`delete_leaf_miss_out_of_leaf=60` plus `delete_leaf_miss_last_cell=3`.

## Optimization Read

This current-head run confirms the previous DML frontier read: the DELETE gap is
still concentrated in the retained physical leaf-run path, not in parser, VDBE,
record assembly, or payload-copy work. The next source-level candidate remains
the broader transaction-local logical DML mutation operator described in
`docs/design/profile-first-optimization-cards-and-proof-packs.md`, not another
standalone retained-leaf search, compactness, duplicate-check, materialization,
or direct-flush micro-patch.
