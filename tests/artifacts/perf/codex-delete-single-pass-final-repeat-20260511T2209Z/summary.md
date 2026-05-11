# DELETE Single-Pass Leaf Compaction

Date: 2026-05-11
Binary: `/tmp/frankensqlite-codex-delete-single-pass-bench/release-perf/comprehensive-bench`
Command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update-delete --no-html`

## Keep Gate

Baseline artifact: `tests/artifacts/perf/codex-current-dml-profile-20260511T205339Z/update-delete.json`
Candidate artifact: `tests/artifacts/perf/codex-delete-single-pass-final-repeat-20260511T2209Z/update-delete.json`

| Scenario | Baseline F median | Candidate F median | Result |
| --- | ---: | ---: | --- |
| 100 rows / delete 5 rows | 0.006863 ms | 0.006863 ms | neutral; new path is below threshold |
| 1000 rows / delete 50 rows | 0.044454 ms | 0.028934 ms | 34.9% faster |
| 10000 rows / delete 500 rows | 0.262732 ms | 0.262922 ms | neutral/noise |

The candidate also reduced the profiled DELETE leaf materialization counters:

| Scenario | Baseline materialize | Candidate materialize | Result |
| --- | ---: | ---: | --- |
| 1000 rows / delete 50 rows | 6 / 9879 ns | 6 / 7273 ns | 26.4% lower |
| 10000 rows / delete 500 rows | 64 / 73639 ns | 64 / 67737 ns | 8.0% lower |

Decision: keep. The change has a strong repeatable 1000-row DELETE win, keeps
the 10k row within noise while lowering the targeted materialization counter,
and does not activate on the 5-row DELETE case.

## Verification

- `cargo fmt --check`
- `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-delete-single-pass-test2 CARGO_BUILD_JOBS=4 cargo check --workspace --all-targets`
- `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-delete-single-pass-test2 CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-delete-single-pass-test2 CARGO_BUILD_JOBS=4 cargo test -p fsqlite-core prepared_direct_delete -- --nocapture --test-threads=1`
- `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-delete-single-pass-test2 CARGO_BUILD_JOBS=4 cargo test -p fsqlite-btree test_table_leaf_delete_run_defragments_large_root_leaf_delete_set -- --nocapture --test-threads=1`
- `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-delete-single-pass-bench CARGO_BUILD_JOBS=4 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

UBS was attempted twice and timed out after 180 seconds both times:

- `timeout 180 ubs crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs`
- `timeout 180 ubs --only=rust crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs`
