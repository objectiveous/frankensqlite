# DELETE DML Profile - 2026-05-14

Run id: `codex-d3ecb8b-dml-profile-20260514T202048Z`
Head: `d3ecb8b784d61aa9474157043862a85b3d408af3`

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-d3ecb8b-dml-profile-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 20 delete compare standard
```

Primary log: `perf-update-delete-10000x20-delete-compare-standard.log`
Environment fingerprint: `fingerprint.txt`

The remote benchmark command reported `exit=0`; the local `rch` wrapper was
stopped after it hung while retrieving the worker `.rch-target` directory. The
benchmark stdout was already captured in the primary log before that retrieval
phase.

## Headline Result

Workload: `perf-update-delete`, `rows=10000`, `iters=20`, `which=delete`,
`engine=compare`, `mode=standard`.

| Engine | Total | Populate | Delete | Per-row DELETE |
| --- | ---: | ---: | ---: | ---: |
| FrankenSQLite | 159 ms | 128 ms | 18 ms | 1852 ns |
| C SQLite | 168 ms | 151 ms | 10 ms | 1073 ns |
| Ratio | 0.94x | 0.85x | 1.73x | 1.73x |

Total time is already favorable because populate is faster, but the DELETE
phase remains the active gap.

## Averaged FrankenSQLite Hotspots

These are means across the 20 profiled FrankenSQLite iterations in the log.

| Counter | Mean |
| --- | ---: |
| `elapsed_us` | 926.5 us |
| `delete_leaf_active_ns` | 287.2 us |
| `delete_leaf_flush_ns` | 147.9 us |
| `delete_leaf_materialize` | 64 calls |
| `delete_leaf_materialize_ns` | 114.4 us |
| `execute_body_ns` | 117.4 us |
| `delete_leaf_search` | 560 calls |
| `delete_leaf_search_ns` | 104.1 us |
| `delete_seek_ns` | 100.8 us |
| `commit_roundtrip_ns` | 56.4 us |
| `delete_leaf_dupcheck` | 500 calls |
| `delete_leaf_dupcheck_ns` | 23.5 us |
| `delete_leaf_compact` | 497 calls |
| `delete_leaf_compact_ns` | 30.8 us |
| `delete_leaf_cellparse` | 497 calls |
| `delete_leaf_cellparse_ns` | 26.0 us |
| `bg_ns` | 19.3 us |

Shape counters were stable: each iteration did 500 direct deletes, 64 leaf-run
flushes, 433 active leaf deletes, and 63 active-run misses. Misses were mostly
`delete_leaf_miss_out_of_leaf=60` plus `delete_leaf_miss_last_cell=3`.

## Optimization Read

The remaining DELETE gap is not dominated by parser, VDBE, record assembly, or
payload-copy counters; those stayed at zero in this direct DELETE lane. The
measured cost is concentrated in the retained leaf-run path:

- active retained leaf mutation and search work
- materializing 64 dirty leaf pages during flush
- commit/cache finish work after the physical mutations

This reinforces the prior design conclusion that small tombstone, compact,
duplicate-check, or direct-flush tweaks are weak candidates unless a fresh matrix
run proves otherwise. The next high-leverage source-level candidate is still a
broader logical DML mutation operator with key-stable read-your-writes and one
publication pass, rather than another physical cell-index micro-patch.
