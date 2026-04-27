# Post-Monotone Sort / Snapshot-Off Profile

## Scope

- Current commit: `132b3c3363cf08b4b76860a9baff8e4a23606b44`
  (`perf(bench): suppress time-travel snapshot clone in perf_update_delete + propagate pragma errors`)
- Clean comparison commit: `ead1009f4a6d1b18c1ea3b17d60e25d6b064cec8`
  (`perf(btree): skip monotone defrag sorts`)
- Scenario: `perf-update-delete 10000 50 both`
- Build profile: `release-perf` with frame pointers and line-table debug symbols

This pass separates three facts that were otherwise easy to conflate:

1. `ead1009f` did not materially move the full mixed update/delete workload.
2. The benchmark-level time-travel snapshot opt-out in `132b3c33` does move it.
3. The remaining committed-code profile is now dominated by direct INSERT and
   B-tree delete defrag/sort work rather than time-travel snapshot cloning.

## Timing

Clean committed `ead1009f` comparison runs, built from a detached clean worktree:

| Run | Total | Populate | Update | Delete | Per-row update | Per-row delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Clean 1 | 1128 ms | 491 ms | 340 ms | 226 ms | 6804 ns | 9068 ns |
| Clean 2 | 1152 ms | 488 ms | 364 ms | 228 ms | 7282 ns | 9157 ns |
| Clean 3 | 1135 ms | 484 ms | 347 ms | 235 ms | 6954 ns | 9433 ns |
| Clean 4 | 1143 ms | 490 ms | 349 ms | 232 ms | 6986 ns | 9304 ns |
| Clean 5 | 1125 ms | 486 ms | 342 ms | 228 ms | 6856 ns | 9135 ns |
| Clean perf run | 1150 ms | 497 ms | 354 ms | 226 ms | 7095 ns | 9065 ns |

Median clean total time: `1135 ms`.

Current `132b3c33` runs with `PRAGMA fsqlite_capture_time_travel_snapshots=false`
inside `perf-update-delete`:

| Run | Total | Populate | Update | Delete | Per-row update | Per-row delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Current 1 | 881 ms | 419 ms | 274 ms | 141 ms | 5480 ns | 5678 ns |
| Current 2 | 909 ms | 429 ms | 286 ms | 147 ms | 5730 ns | 5912 ns |
| Current 3 | 930 ms | 433 ms | 301 ms | 146 ms | 6020 ns | 5855 ns |
| Current 4 | 899 ms | 425 ms | 280 ms | 149 ms | 5619 ns | 5967 ns |
| Current 5 | 866 ms | 414 ms | 276 ms | 133 ms | 5528 ns | 5331 ns |
| Current perf run | 879 ms | 416 ms | 279 ms | 142 ms | 5581 ns | 5715 ns |

Median current total time: `899 ms`.

Delta from clean `ead1009f` median to current `132b3c33` median:
`1135 ms -> 899 ms`, about `20.8%` faster for this benchmark scenario.

## Profile Shift

Clean `ead1009f` flat profile top:

| Overhead | Symbol / interpretation |
| ---: | --- |
| 6.30% | `__memmove_avx_unaligned_erms` |
| 4.38% | `_int_malloc`, including `capture_time_travel_snapshot` `MemDatabase` clone |
| 4.28% | `sort_cells_desc_by_ptr` quicksort path |
| 3.90% | `BtCursor<SharedTxnPageIo>::delete` |
| 3.75% | `Connection::execute_prepared_direct_simple_insert` |
| 3.30% | `cell_on_page_size_fast` |

Current `132b3c33` flat profile top:

| Overhead | Symbol / interpretation |
| ---: | --- |
| 7.58% | `__memmove_avx_unaligned_erms`, including delete defrag copying |
| 6.91% | `Connection::execute_prepared_direct_simple_insert` |
| 5.43% | `sort_cells_desc_by_ptr` quicksort path |
| 5.03% | `cell_on_page_size_fast` |
| 4.23% | `BtCursor<SharedTxnPageIo>::delete` |
| 3.13% | `_int_malloc`, now mostly direct-insert/B-tree allocation rather than snapshot clone |

The snapshot-off commit removes the visible `capture_time_travel_snapshot` clone
from the benchmark profile. The next committed-code opportunities are now direct
INSERT append/value construction and B-tree DELETE defrag.

## B-tree Sort Probe

The existing ignored microbench does not support another monotone-sort tweak as
the next safe lever:

| Shape | std | dispatched | Speedup |
| --- | ---: | ---: | ---: |
| N=1 | 8 ns | 8 ns | 1.00x |
| N=2 | 9 ns | 8 ns | 1.12x |
| N=4 | 10 ns | 9 ns | 1.11x |
| N=8 | 12 ns | 10 ns | 1.20x |
| N=12 | 16 ns | 14 ns | 1.14x |
| N=16 | 28 ns | 19 ns | 1.47x |
| N=32 | 24 ns | 35 ns | 0.69x |
| N=60 | 50 ns | 98 ns | 0.51x |
| N=80 | 60 ns | 96 ns | 0.62x |

That makes the large-N sort path a bad immediate target without a different
algorithm or a workload-shaped counter proving common non-std-friendly input.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Decision |
| --- | ---: | ---: | ---: | ---: | --- |
| Keep committed benchmark snapshot opt-out | 4 | 5 | 1 | 20.0 | Landed by `132b3c33`; this artifact verifies the win |
| Direct INSERT value/text construction | 3 | 3 | 3 | 3.0 | Next source lane |
| B-tree DELETE defrag copy/cell-size path | 3 | 3 | 3 | 3.0 | Next source lane, but avoid another sort-only tweak |
| Engine-level lazy/differential time-travel snapshots | 5 | 2 | 5 | 2.0 | Bigger design task; preserve `FOR SYSTEM_TIME` behavior |
| More monotone-sort specialization | 2 | 1 | 2 | 1.0 | Reject for now; microbench is negative at N>=32 |

## Commands

```bash
git worktree add --detach /data/tmp/frankensqlite-azurepine-clean-20260427T1603 ead1009f
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-clean-ead CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

```bash
perf record -F 997 -g --call-graph dwarf -o tests/artifacts/perf/20260427T1603Z-azurepine-post-monotone-sort-profile/clean-head-perf.data -- /data/tmp/cargo-target-azurepine-20260427-clean-ead/release-perf/perf-update-delete 10000 50 both
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-post-monotone CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

```bash
perf record -F 997 -g --call-graph dwarf -o tests/artifacts/perf/20260427T1603Z-azurepine-post-monotone-sort-profile/dirty-snapshot-off-perf.data -- /data/tmp/cargo-target-azurepine-20260427-post-monotone/release-perf/perf-update-delete 10000 50 both
```

```bash
env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-btree-sort cargo test -p fsqlite-btree bench_remove_cell_from_leaf_sort --profile release-perf -- --ignored --nocapture
```
