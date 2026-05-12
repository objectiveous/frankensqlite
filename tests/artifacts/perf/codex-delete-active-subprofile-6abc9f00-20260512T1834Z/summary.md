# Retained DELETE Leaf-Run Subprofile

Date: 2026-05-12

Base commit before this instrumentation: `6abc9f007e1cd2044a9c5d74244dc5f0fedcbc11`

## Commands

Profile-enabled run:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 20 delete compare standard
```

Profile-off guard run:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 20 delete compare standard
```

## Results

Profile-enabled result:

- FSQLite: `83ms` total, `9ms` delete, `953ns` per delete row.
- C SQLite: `71ms` total, `3ms` delete, `384ns` per delete row.
- Ratio: `1.17x` total, `2.48x` delete.
- Retained DELETE run shape stayed stable: `500/0` parser fast/slow, `433/496` active same-leaf hits, `64/64` flushes.

Average profile counters across 20 FSQLite iterations:

- `delete_leaf_active_ns`: `147940.0`
- `delete_leaf_search`: `560.0` calls, `43709.8ns`
- `delete_leaf_dupcheck`: `500.0` calls, `12776.0ns`
- `delete_leaf_compact`: `497.0` calls, `15792.7ns`
- `delete_leaf_cellparse`: `497.0` calls, `13150.1ns`
- `delete_leaf_flush_ns`: `82961.8`
- `delete_leaf_materialize`: `64.0` calls, `63961.4ns`
- `delete_leaf_write`: `64.0` calls, `12992.4ns`
- `delete_seek_ns`: `54738.3`
- `delete_physical_ns`: `15811.0`

Profile-off guard result:

- FSQLite: `67ms` total, `5ms` delete, `528ns` per delete row.
- C SQLite: `69ms` total, `3ms` delete, `361ns` per delete row.
- Ratio: `0.96x` total, `1.46x` delete.
- No `dml_profile` line was emitted without `FSQLITE_BENCH_PROFILE_DML=1`.

## Read

The retained same-leaf DELETE active bucket is dominated by rowid search inside
the leaf-run image. The smaller same-row checks are still visible but are not
large enough to justify another isolated micro-optimization without changing
the overall DELETE operator shape. A plausible next lever is to avoid repeated
in-leaf search for monotone same-leaf retained DELETE runs, but that must be
tested against the exact 10k/20 standard DELETE row before keeping it.
