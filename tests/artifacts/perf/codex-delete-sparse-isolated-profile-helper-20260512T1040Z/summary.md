# Sparse Isolated DELETE Profiler Helper

- Date: 2026-05-12
- Git before patch: `2a288ae4d1cd9db56cbcc65a47802419f656d49d`
- Target: `crates/fsqlite-e2e/src/bin/perf_update_delete.rs`

## Finding

The existing `isolated` DELETE profiling mode does not preserve the
`comprehensive-bench` sparse DELETE shape. Standard Section 6 deletes rowids
`i * 20` inside each benchmark table, but `isolated` used contiguous unique
rowids across iterations. That can manufacture page-drain / balance hot spots
that are not representative of the red standard DELETE rows.

## Patch

Added a `sparse-isolated` mode that keeps the benchmark-shaped sparse rowids
inside each logical block:

```text
rowid = iter * rows + i * 20
```

This preserves the sparse per-block DELETE shape while still avoiding repeated
deletes of the same rowids.

## Smoke Evidence

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-sparse-profiler \
  CARGO_BUILD_JOBS=8 \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 100 delete compare sparse-isolated
```

Result:

- FSQLite: total `388ms`, populate `327ms`, delete `52ms`,
  per-row-delete `1056ns`.
- C SQLite: total `381ms`, populate `357ms`, delete `22ms`,
  per-row-delete `448ns`.
- F/C delete ratio: `2.36x`.

This smoke is a profiling-helper sanity check, not a replacement for
`comprehensive-bench --quick --filter update`. The primary benchmark frontier
remains the standard DML matrix.

## Verification

- `cargo test -p fsqlite-e2e --bin perf-update-delete -- --nocapture`
- `cargo fmt --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `ubs crates/fsqlite-e2e/src/bin/perf_update_delete.rs`

Agent Mail registration/reservation was attempted first but the local MCP
server at `127.0.0.1:8765` was unreachable, so no reservation could be created.
