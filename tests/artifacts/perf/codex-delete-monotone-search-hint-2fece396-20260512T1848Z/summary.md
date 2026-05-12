# Rejected Retained DELETE Monotone Search Hint

Date: 2026-05-12

Base commit before rejected candidate: `2fece3966b1200528256a480bf99d55613c98f2e`

## Commands

Profile-off compare observed before unwinding the candidate patch:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 20 delete compare standard
```

Profile-enabled compare captured in `delete-standard-profile.stderr.txt`:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 20 delete compare standard
```

## Candidate

The abandoned source patch added a lower-bound hint to
`TableLeafDeleteRun::search_table_leaf`, attempting to continue increasing
same-leaf DELETE rowid probes from the prior insertion point while falling back
to binary search for out-of-order rowids.

The patch was unwound before commit.

## Results

Profile-off candidate result observed before unwinding:

- FSQLite: `65ms` total, `5ms` delete, `551ns` per delete row.
- C SQLite: `68ms` total, `3ms` delete, `346ns` per delete row.
- Ratio: `0.96x` total, `1.59x` delete.

The raw profile-off stderr was not retained because the file was overwritten by
a post-unwind guard run. The rejection does not depend on that file; the
profile-enabled candidate raw output below captures the failed sub-counter.

Profile-enabled candidate result:

- FSQLite: `84ms` total, `9ms` delete, `936ns` per delete row.
- C SQLite: `69ms` total, `3ms` delete, `344ns` per delete row.
- Ratio: `1.22x` total, `2.73x` delete.

Average profile counters across 20 FSQLite iterations:

- `delete_leaf_active_ns`: `164208.0`
- `delete_leaf_search`: `560.0` calls, `58736.6ns`
- `delete_leaf_dupcheck`: `500.0` calls, `12591.4ns`
- `delete_leaf_compact`: `497.0` calls, `15613.8ns`
- `delete_leaf_cellparse`: `497.0` calls, `12918.9ns`
- `delete_leaf_flush_ns`: `78894.4`
- `delete_leaf_materialize`: `64.0` calls, `59639.7ns`
- `delete_leaf_write`: `64.0` calls, `13417.6ns`
- `delete_seek_ns`: `48969.8`
- `delete_physical_ns`: `14832.0`

The previous profile artifact
`tests/artifacts/perf/codex-delete-active-subprofile-6abc9f00-20260512T1834Z/`
measured `delete_leaf_search` at `43709.8ns` and the profile-off guard row at
`528ns` per FSQLite delete. This candidate worsened the intended sub-counter
and did not improve the exact no-profile benchmark row.

## Decision

Rejected. Do not retry a standalone monotone retained-leaf search hint. The
extra sequential lower-bound path loses to the existing binary search on this
workload shape.
