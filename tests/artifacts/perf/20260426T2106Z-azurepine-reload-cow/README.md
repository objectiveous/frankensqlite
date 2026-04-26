# Memdb Reload Row/Payload Fusion

Date: 2026-04-26
Agent: AzurePine
Commit base before patch: dc6e5471

## Change

`reload_memdb_from_txn_with_mode` now uses `cursor.rowid_and_payload_cow(cx)?`
for sqlite_master and hydrated table row scans instead of parsing the same leaf
cell once for `rowid()` and again for `payload()`.

## Benchmark

Command:

```bash
/data/tmp/cargo-target-azurepine-reload-cow/release-perf/perf-update-delete 10000 50 both
```

Build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-reload-cow CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Results:

| Tree | Run | total | populate | update | delete | per-row update | per-row delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 1 | 1503ms | 609ms | 466ms | 335ms | 9327ns | 13434ns |
| baseline | 2 | 1381ms | 558ms | 431ms | 311ms | 8629ns | 12461ns |
| patched | 1 | 1093ms | 480ms | 335ms | 210ms | 6700ns | 8431ns |
| patched | 2 | 1133ms | 487ms | 341ms | 227ms | 6840ns | 9081ns |

Baseline median: 1442ms.
Patched median: 1113ms.
Observed median delta: -22.8%.

## Verification

- `cargo fmt --check --package fsqlite-core`
- `cargo fmt --check`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-reload-cow-check cargo test -p fsqlite-core reload_memdb_from_pager -- --nocapture`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-reload-cow-check cargo check --workspace --all-targets`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-reload-cow-check cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check -- crates/fsqlite-core/src/connection.rs`

UBS note: `ubs crates/fsqlite-core/src/connection.rs` was attempted, but it
stalled inside the Rust module's `ast-grep run --pattern '$X as i32'` subcheck
against the shadow workspace and was stopped after several minutes with no
findings emitted.
