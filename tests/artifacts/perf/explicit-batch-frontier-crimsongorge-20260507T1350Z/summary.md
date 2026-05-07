# Explicit Batch Frontier Right-Edge Bulk Append

Agent: CrimsonGorge
Base commit before candidate: `60e1434f5d001180ce4258e1dfb34be55c06036f`
Working tree state during measurement: dirty with the candidate source changes.

## Change

- Added `BtCursor::table_bulk_append_depth2_right_edge_sorted_records`, a narrow table-B-tree primitive for sorted right-edge appends into an existing depth-2 tree.
- Added `BtCursor::table_can_bulk_append_depth2_right_edge_record` and used it as the connection-layer admission guard. Non-empty buffering now starts only when the current B-tree already matches the depth-2 right-edge page-builder shape, the new rowid is above the current right edge, the first record is below the large-record cutoff, and no savepoint is active.
- Flush order is now:
  1. empty-root bulk load,
  2. depth-2 right-edge bulk append,
  3. existing row-by-row append replay fallback.

## Target Result

Baseline evidence from `../transaction-profile-after-leafhint-crimsongorge-20260507T1345Z/summary.md`:

- `10000 rows / batched (1000/txn)`: C `3.107059 ms`, F `4.521009 ms`, ratio `1.455077`.
- Profile showed `cursor_setup_ns=395141`, `btree_insert_ns=1504173`, `btree_leaf_payload_appends=8934`, and `btree_quick_balance_hits=57`.

Final narrowed candidate after rebuilding the release-perf binary:

- `final-transaction.json`
- `10000 rows / batched (1000/txn)`: C `3.126989 ms`, F `2.540469 ms`, ratio `0.812433`.
- Profile in `stdout/final-transaction.err` shows `cursor_setup_ns=11321`, `btree_insert_ns=305619`, `btree_leaf_payload_appends=0`, and `btree_quick_balance_hits=0`.

The target row moved from about `1.46x` slower than C SQLite to about `1.23x` faster than C SQLite.

## Insert-Section Check

`final-insert.json` was run after narrowing admission.

- Scenarios: 25
- Franken faster / comparable / C faster: `13 / 2 / 10`
- Geomean ratio: `0.932698`
- Median ratio: `0.930170`
- P90 ratio: `1.206028`
- Weighted score: `0.878437`
- Write-bulk geomean: `0.943407`
- Write-single geomean: `0.857791`

Comparison points from TanBear's same-window INSERT repeat:

- Clean insert weighted score: `0.902771`
- Broad dirty insert weighted score: `0.921972`

The narrowed source-owned insert run clears the broad-admission warning: it keeps the target row win and improves the INSERT weighted score versus both the clean and broad dirty repeats.

## Full-Matrix Check

`final-full-repeat.json` was the governing full quick rerun after an earlier final full run produced a non-reproducing concurrent-writer outlier.

- Scenarios: 93
- Franken faster / comparable / C faster: `75 / 5 / 13`
- Geomean ratio: `0.284476`
- Median ratio: `0.293639`
- P90 ratio: `1.133819`
- P99 ratio: `1.509348`
- Weighted score: `0.363457`

Comparison baseline: `../full-refresh-after-leafhint-crimsongorge-20260507T1320Z/report-full.json`.

- Baseline weighted score: `0.363259`
- Baseline geomean ratio: `0.281742`
- Baseline median ratio: `0.294272`
- Baseline P90 ratio: `1.140709`
- Baseline P99 ratio: `1.453092`
- Baseline `10000 rows / batched (1000/txn)`: ratio `1.453092`, F `4.587983 ms`
- Candidate `10000 rows / batched (1000/txn)`: ratio `0.801765`, F `2.482000 ms`

The full-score delta is effectively flat against the older full-refresh artifact and better than TanBear's same-window clean full quick (`0.370335`). The isolated target row, final transaction rerun, and final insert matrix show the intended step change without the broad-admission source shape.

## Outlier Check

The first full candidate run reported `large_10col` record-size at about `21 ms`, which would have been a reject signal if reproducible. A targeted record-size rerun with profiling (`candidate-record-profile.json`) did not reproduce it:

- `large_10col` record-size rerun: C `9.303309 ms`, F `11.597456 ms`, ratio `1.246595`.
- Profile showed the same empty-root page-run counters, with no leaf payload append path.

That row is not reachable from the new non-empty buffering logic in a single empty-table transaction, so it was treated as benchmark noise rather than candidate causality.

The first final full run also reported `8 writers x 1000 rows` at ratio `1.997543`. `final-full-repeat.json` did not reproduce that concurrent-writer outlier; the repeat reported concurrent-writer geomean `0.762819` and kept the full weighted score near the current full-refresh baseline.

## Verification

- `cargo fmt -p fsqlite-btree -p fsqlite-core --check`
- `git diff --check -- crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs`
- `rch exec -- env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-explicit-batch-check-target CARGO_BUILD_JOBS=16 cargo check --workspace --all-targets`
- `rch exec -- env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-explicit-batch-check-target CARGO_BUILD_JOBS=16 cargo clippy --workspace --all-targets -- -D warnings`
- `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-explicit-batch-check-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-btree test_table_bulk_append_depth2_right_edge_sorted_records_extends_tree -- --nocapture`
- `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-explicit-batch-check-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_prepared_direct_insert_page_run_buffers_nonempty_right_edge_batch -- --nocapture`
- `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-explicit-batch-perf-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
- `FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-explicit-batch-perf-target/release-perf/comprehensive-bench --quick --filter transaction --json-out .../final-transaction.json --no-html`
- `/data/tmp/frankensqlite-explicit-batch-perf-target/release-perf/comprehensive-bench --quick --filter insert --json-out .../final-insert.json --no-html`
- `/data/tmp/frankensqlite-explicit-batch-perf-target/release-perf/comprehensive-bench --quick --json-out .../final-full-repeat.json --no-html`

UBS was attempted on the two touched Rust files, but the tool-side Rust scanner stalled in an `ast-grep` subscan for several minutes and exited from its cleanup path after interruption without producing findings. This artifact does not treat UBS as a completed verification gate.
