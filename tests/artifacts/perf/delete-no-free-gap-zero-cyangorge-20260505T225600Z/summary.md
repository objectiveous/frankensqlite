# Table-leaf DELETE unused-gap zero-fill removal

Agent: CyanGorge
Date: 2026-05-05

## Candidate

Removed the optional `page_bytes[ptr_array_end..new_content_offset].fill(0)`
from the three `remove_table_cell_from_leaf_deferred` table-leaf compaction
paths in `crates/fsqlite-btree/src/cursor.rs`.

The live B-tree state is still defined by:

- `first_freeblock = 0`
- `fragmented_free_bytes = 0`
- `cell_content_offset = new_content_offset`
- the rewritten cell pointer array

The removed writes only zeroed the newly unreachable gap between the pointer
array and compacted cell-content region. Inserts overwrite new cell bytes before
publishing their pointers, so the stale gap bytes are not reachable through the
B-tree cursor.

## Focused isolated harness

Command shape:

```bash
.rch-target/release-perf/perf-update-delete 10000 1000 delete fsqlite isolated
.rch-target/release-perf/perf-update-delete 10000 250 both fsqlite isolated
```

Results:

| workload | baseline | candidate | result |
| --- | ---: | ---: | --- |
| delete-only delete phase | 1189 ms | 945 ms | 1.26x faster |
| delete-only total | 1535 ms | 1289 ms | 1.19x faster |
| mixed delete phase | 194 ms | 185 ms | 1.05x faster |
| mixed total | 560 ms | 562 ms | flat |

## Comprehensive update/delete section

Baseline:
`tests/artifacts/perf/update-delete-current-cyangorge-20260505T225600Z/profile-report.json`

Candidate:
`tests/artifacts/perf/delete-no-free-gap-zero-cyangorge-20260505T225600Z/comprehensive-candidate-report.json`

| row | baseline ratio | candidate ratio | baseline F ms | candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 4.0521 | 3.9268 | 0.354213 | 0.345618 |
| 1000 rows / delete 50 rows | 2.6741 | 2.3983 | 1.106863 | 0.907239 |
| 10000 rows / delete 500 rows | 2.5092 | 2.3475 | 8.497694 | 8.051460 |

The filtered section geomean moved `3.0545 -> 3.0050`. The section average
ratio was noisy because the 100-row UPDATE row moved against the candidate even
though this patch touches only DELETE compaction.

## Full quick matrix

Candidate:
`tests/artifacts/perf/delete-no-free-gap-zero-cyangorge-20260505T225600Z/full-quick-candidate-report.json`

The full quick matrix did not improve the primary weighted score:

- previous full quick score: `0.5553`
- candidate full quick score: `0.5624`

The drift is in unrelated read/concurrent rows plus noisy C SQLite medians. This
candidate is kept only as a narrow DELETE-path win, not as a broad matrix score
improvement.

## Correctness and build checks

- `cargo fmt --check`
- `rch exec -- env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree cursor_delete -- --nocapture`
  passed all 7 focused cursor-delete tests before `rch` target artifact
  retrieval was interrupted after the successful test result.
- `env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree cursor_delete -- --nocapture`
- `env CARGO_TARGET_DIR=.rch-target cargo check -p fsqlite-btree -p fsqlite-e2e --all-targets`
- `env CARGO_TARGET_DIR=.rch-target cargo clippy -p fsqlite-btree --all-targets -- -D warnings`
- `git diff --check`
- `ubs crates/fsqlite-btree/src/cursor.rs tests/artifacts/perf/delete-no-free-gap-zero-cyangorge-20260505T225600Z/summary.md`
  exited 0. UBS still reported broad pre-existing heuristic warnings in
  `cursor.rs`, but no critical findings and its cargo/rustfmt/clippy checks
  were clean.
