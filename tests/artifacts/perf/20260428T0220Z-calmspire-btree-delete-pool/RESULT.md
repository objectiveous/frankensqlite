# B-tree table-delete pointer-pool A/B

Run id: `20260428T0220Z-calmspire-btree-delete-pool`
Agent: `CalmSpire`
Head after peer core/WAL lane landed: `ea2cf2812b04775407695f9f821ffaeb462da142`

## Scope

Tested a one-file `crates/fsqlite-btree/src/cursor.rs` candidate that changes
`remove_table_cell_from_leaf_deferred` to reuse the existing pooled cell-pointer
buffer and the cursor defrag scratch buffer instead of allocating fresh vectors
on table-leaf delete.

At measurement time, the worktree also contained peer-owned dirty edits in:

- `crates/fsqlite-core/src/connection.rs`
- `crates/fsqlite-core/src/wal_adapter.rs`

Those files were held constant in both baseline and candidate builds. During
validation they landed as `ea2cf281`, so the final commit parent already
contains them. This A/B isolates only the `cursor.rs` patch.

Patch file:

- `/data/tmp/calmspire-btree-delete-pool.patch`
- SHA-256: `35bf562f0630613748982e7112cc790726a5c3f09e26fc773b3069d526c256b2`

## Build

Baseline:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-calmspire-20260428-btree-delete-baseline cargo build -p fsqlite-e2e --profile release-perf --bin perf-update-delete
```

Candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-calmspire-20260428-btree-delete-candidate cargo build -p fsqlite-e2e --profile release-perf --bin perf-update-delete
```

## Measurement

```bash
hyperfine --warmup 2 --runs 10 --export-json tests/artifacts/perf/20260428T0220Z-calmspire-btree-delete-pool/hyperfine.json --command-name baseline-delete '/data/tmp/cargo-target-calmspire-20260428-btree-delete-baseline/release-perf/perf-update-delete 10000 100 delete' --command-name candidate-delete '/data/tmp/cargo-target-calmspire-20260428-btree-delete-candidate/release-perf/perf-update-delete 10000 100 delete' --command-name baseline-both '/data/tmp/cargo-target-calmspire-20260428-btree-delete-baseline/release-perf/perf-update-delete 10000 100 both' --command-name candidate-both '/data/tmp/cargo-target-calmspire-20260428-btree-delete-candidate/release-perf/perf-update-delete 10000 100 both'
```

Results:

| Scenario | Baseline mean | Candidate mean | Delta |
| --- | ---: | ---: | ---: |
| `perf-update-delete 10000 100 delete` | 0.976579805 s | 0.940093362 s | 3.736% faster |
| `perf-update-delete 10000 100 both` | 1.271343815 s | 1.260631060 s | 0.843% faster |

Raw JSON:

- `hyperfine.json`
- SHA-256: `49d492f6886622c1a3ea15cf0df7310462d926af09eecb3ee6623acaf1240003`

## Verification

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-calmspire-20260428-btree-delete cargo test -p fsqlite-btree delete -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-calmspire-20260428-gate cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-calmspire-20260428-gate cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-btree/src/cursor.rs tests/artifacts/perf/20260428T0220Z-calmspire-btree-delete-pool/RESULT.md tests/artifacts/perf/20260428T0220Z-calmspire-btree-delete-pool/hyperfine.json
```

Focused test result: 30 passed, 0 failed.
Workspace check and clippy both passed. UBS reported no critical issues and
exited 0; its warnings were pre-existing broad-file heuristics in
`cursor.rs`.
