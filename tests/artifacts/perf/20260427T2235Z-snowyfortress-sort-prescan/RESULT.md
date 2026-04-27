# Packed Table-Leaf Delete Gap Shift

Date: 2026-04-27
Agent: SnowyFortress
Base revision: 6e0fd73d

## Profile Signal

Fresh `perf-update-delete 10000 100 both` profile:

- `__memmove_avx_unaligned_erms`: 8.14% self
- `BtCursor::delete`: 6.63% self
- `sort_cells_desc_by_ptr` quicksort path: 5.47% self
- `small_sort_general` sort path: 2.96% self
- `read_cell_pointers_into`: 1.24% self

## Candidates

1. Rejected: remove the large-N monotonic pre-scan in `sort_cells_desc_by_ptr`.
   - Local sort microbench improved N > 20 mixed cases.
   - End-to-end `both` regressed within noise: base 1.566s, candidate 1.578s.
   - Delete-only was only 1.01x +/- 0.03, below the threshold for keeping code.

2. Accepted: for compact, descending table-leaf pages, delete the packed cell by shifting the single lower contiguous block once and adjusting later cell pointers.
   - This replaces per-surviving-cell `copy_within` calls on the common packed-page delete path.
   - The fallback non-compact and mixed-order path is unchanged.

## Measurements

`hyperfine-gapshift-10000x100-both.json`:

- base-6e0fd73: 1.590s +/- 0.058s
- candidate-packed-gap-shift: 1.561s +/- 0.037s
- result: candidate 1.02x +/- 0.04 faster

`hyperfine-gapshift-10000x100-delete.json`:

- base-6e0fd73: 994.0ms +/- 18.7ms
- candidate-packed-gap-shift: 980.2ms +/- 24.1ms
- result: candidate 1.01x +/- 0.03 faster

`hyperfine-gapshift-10000x100-delete-20runs.json`:

- base-6e0fd73: 963.9ms +/- 11.9ms
- candidate-packed-gap-shift: 921.9ms +/- 22.7ms
- result: candidate 1.05x +/- 0.03 faster

## Verification

- `cargo fmt -p fsqlite-btree --check`: pass
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-gap-main-check cargo check -p fsqlite-btree --all-targets`: pass
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-gap-main-check cargo clippy -p fsqlite-btree --all-targets -- -D warnings`: pass
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-gap-main-test cargo test -p fsqlite-btree delete -- --nocapture`: 29 passed
- `cargo fmt --check`: pass
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-gap-workspace cargo check --workspace --all-targets`: pass
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-gap-workspace cargo clippy --workspace --all-targets -- -D warnings`: pass
- `timeout 180s ubs crates/fsqlite-btree/src/cursor.rs tests/artifacts/perf/20260427T2235Z-snowyfortress-sort-prescan/RESULT.md`: exit 0, no critical issues; broad pre-existing warnings in the large test-heavy cursor file remain.
