# B-tree Delete Sort Record Narrowing Rejection

## Scenario

- Workload: `perf-update-delete 10000 {50,200,500} both`
- Baseline binary: `ca88edde` (`fix(vfs): add FreeBSD l_sysid field to flock initializers`)
- Candidate: local one-lever patch on top of `ca88edde`
- Publication head: `26d22935` (`fix(vfs): reject overflowing Unix I/O offsets`) landed while the rejection was being recorded
- Host: `threadripperje`, AMD Ryzen Threadripper PRO 5995WX, 128 logical CPUs
- Kernel: `Linux 6.17.0-19-generic`
- Toolchain: `rustc 1.97.0-nightly (ca9a134e0 2026-04-26)`
- Build: `release-perf`, `CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only`, `CARGO_PROFILE_RELEASE_PERF_STRIP=false`, `RUSTFLAGS='-C force-frame-pointers=yes'`

## Profile Input

Current-head profile before the candidate showed B-tree leaf delete sort as the next apparent lever:

- `core::slice::sort::unstable::quicksort...sort_unstable_by<sort_cells_desc_by_ptr>`: `4.67%` self
- `core::slice::sort::shared::smallsort::small_sort_general...sort_unstable_by<sort_cells_desc_by_ptr>`: `4.21%` self
- Call graph attributed the hot sort to `remove_table_cell_from_leaf_deferred` under both direct UPDATE and DELETE:
  - UPDATE path: `remove_table_cell_from_leaf_deferred` `9.49%`, including `sort_cells_desc_by_ptr` `6.33%`
  - DELETE path: `remove_table_cell_from_leaf_deferred` `4.34%`, including `sort_cells_desc_by_ptr` `2.59%`

Opportunity score before measurement:

| Hotspot | Impact | Confidence | Effort | Score |
|---|---:|---:|---:|---:|
| `sort_cells_desc_by_ptr` move width | 4 | 3 | 1 | 12.0 |

The alien-artifact/graveyard framing reduced this to a compiled-runtime-kernel idea: keep the same sort key/order, but reduce the carried record width instead of changing B-tree semantics or adding a new data structure.

## Candidate

Rejected patch:

- Replaced `(usize, usize, usize)` move triples with a compact `CellMove { ptr: usize, size: u32, index: u32 }`.
- Kept `ptr` as the primary descending sort key.
- Kept the compact-page skip sentinel with a narrowed index sentinel.
- Updated the sort tests and defrag call sites mechanically.

Behavior proof obligations were satisfied by construction:

- Ordering preserved: yes, sort key stayed descending physical cell offset.
- Tie-breaking unchanged: effectively irrelevant; cell offsets are unique on valid pages, and the helper remains unstable.
- Floating-point: N/A.
- RNG seeds: N/A.
- Golden behavior: focused sort equivalence test passed.

Focused verification before timing:

```text
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-btree-sort-record CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo test -p fsqlite-btree remove_cell_from_leaf_specialized_sort_matches_std -- --nocapture
```

Result: 1 focused test passed.

## A/B Results

Baseline binary:

```text
/data/tmp/cargo-target-azurepine-20260427-next2-profile/release-perf/perf-update-delete
```

Candidate binary:

```text
/data/tmp/cargo-target-azurepine-20260427-btree-sort-record/release-perf/perf-update-delete
```

Short alternating run:

| Run | total | populate | update | delete |
|---|---:|---:|---:|---:|
| baseline 50 run 1 | 818 ms | 419 ms | 238 ms | 124 ms |
| candidate 50 run 1 | 832 ms | 416 ms | 252 ms | 121 ms |
| baseline 50 run 2 | 804 ms | 412 ms | 233 ms | 123 ms |
| candidate 50 run 2 | 802 ms | 405 ms | 238 ms | 122 ms |
| baseline 200 | 3246 ms | 1687 ms | 926 ms | 492 ms |
| candidate 200 | 3197 ms | 1599 ms | 948 ms | 507 ms |

Longer target check:

| Run | total | populate | update | delete |
|---|---:|---:|---:|---:|
| baseline 500 | 7885 ms | 4141 ms | 2348 ms | 1077 ms |
| candidate 500 | 7902 ms | 3980 ms | 2372 ms | 1199 ms |

## Decision

Rejected and rolled back.

The candidate did not improve the target path. In the longest check, total time was effectively flat-to-slower (`7885 ms -> 7902 ms`), while target delete time regressed by `122 ms` (`1077 ms -> 1199 ms`, about `11.3%`) and update time regressed by `24 ms` (`2348 ms -> 2372 ms`, about `1.0%`).

Do not retry this exact narrowing as an optimization. The added field access/cast shape loses on the measured workload despite reducing the nominal sort record width.

## Next Target

Re-profile from publication head `26d22935` before selecting the next lever. The likely remaining candidates are still:

- avoid or reduce `sort_cells_desc_by_ptr` work in `remove_table_cell_from_leaf_deferred` without changing target record representation;
- direct INSERT expression/value handling;
- page copy/write paths under `SharedTxnPageIo`.
