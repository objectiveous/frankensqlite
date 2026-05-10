# Bulk Leaf Layout Cache Candidate

Date: 2026-05-10

Baseline: `98aee4f8ddaf0f2e1c6076b1ade4e5b4c574a030`

Candidate: dirty worktree patch to `crates/fsqlite-btree/src/cursor.rs` that
cached table-leaf cell lengths for `records.len() >= 512` empty-root bulk loads.
The source patch was rejected and reverted.

## Commands

- Candidate proof:
  `cargo fmt -p fsqlite-btree --check`
- Candidate proof:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-candidate-22361625-local-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-btree table_bulk -- --nocapture --test-threads=1`
- Candidate build:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-candidate-22361625-local-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
- Baseline build from archive checkout:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-baseline-98aee4f8-layout-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
- Baseline benchmark:
  `/data/tmp/frankensqlite-baseline-98aee4f8-layout-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/codex-bulk-leaf-layout-cache-20260510T212616Z/baseline-insert.json --no-html`
- Candidate benchmark:
  `/data/tmp/frankensqlite-candidate-22361625-local-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/codex-bulk-leaf-layout-cache-20260510T212616Z/candidate-insert.json --no-html`

## Result

Rejected. The exact target rows regressed:

| Row | Baseline FSQLite median | Candidate FSQLite median |
| --- | ---: | ---: |
| `large_10col` 10K single transaction | `10.162639 ms` | `11.463003 ms` |
| record-size `large_10col` 10K | `9.602149 ms` | `11.860357 ms` |
| `medium_6col` 10K single transaction | `3.598417 ms` | `3.900554 ms` |
| `small_3col` 10K single transaction | `2.663658 ms` | `2.794452 ms` |

The likely problem is that caching the cell length alone saves only grouping
length recomputation while still paying the later varint rewrite and adding a
large layout vector. Do not retry this standalone; the next viable INSERT lever
needs to fuse record-body construction with page layout/materialization.
