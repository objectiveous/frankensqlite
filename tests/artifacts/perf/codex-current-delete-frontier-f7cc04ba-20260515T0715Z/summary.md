# Current DELETE Frontier Recheck

- Date: 2026-05-15
- Source checkout: `f7cc04ba` (`fix(core): harden alter table rename schema updates`)
- Build command:
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-campaign-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete`
- Benchmark binary:
  - `/data/tmp/frankensqlite-perf-campaign-target/release-perf/perf-update-delete`

## No-Profile Compare Runs

| Command | FSQLite delete | C SQLite delete | F/C delete |
|---|---:|---:|---:|
| `perf-update-delete 100 500 delete compare standard` | `1476 ns/row` | `420 ns/row` | `3.51x` |
| `perf-update-delete 1000 200 delete compare standard` | `552 ns/row` | `326 ns/row` | `1.70x` |
| `perf-update-delete 10000 100 delete compare standard` | `538 ns/row` | `344 ns/row` | `1.56x` |

The current checkout still matches the published frontier shape: DELETE remains
red, with the small 5-row DELETE tail worst and the 500-row row still red but
stable enough to profile.

## Profile Sample

Profile command:

```text
FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-perf-campaign-target/release-perf/perf-update-delete 10000 20 delete fsqlite standard
```

Representative steady-state samples kept every DELETE on the prepared direct
path (`direct_delete=500`, `slow=0`). The retained leaf-run shape did not
change:

- `delete_leaf_start=64/67`
- `delete_leaf_active=433/496`
- `delete_leaf_miss=63`
- `delete_leaf_miss_out_of_leaf=60`
- `delete_leaf_miss_last_cell=3`
- `delete_leaf_flush=64/64`
- `delete_leaf_search=560`
- `delete_leaf_dupcheck=500`
- `delete_leaf_compact=497`
- `delete_leaf_cellparse=497`

Representative steady-state time buckets were still concentrated in active
same-leaf deletes, retained-run flush/materialization, and commit/cache-finish
work. This does not invalidate the existing negative-result fences around
standalone `TableLeafDeleteRun` search, admission, materialization,
direct-flush, cancellation-polling, and tombstone-only overlay tweaks.
