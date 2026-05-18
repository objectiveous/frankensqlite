# Prepared Direct Logical DELETE Candidate - Rejected

- Date: 2026-05-18
- Scratch checkout: `/tmp/frankensqlite-clean-20260518-ops`
- Candidate: transaction-local logical rowid DELETE messages for private
  `:memory:` prepared direct DELETE, with physical B-tree mutation deferred to
  read/savepoint/commit boundaries.
- Correctness checks:
  - `cargo fmt --check`
  - `cargo test -p fsqlite-core --lib logical_delete -- --nocapture`
  - `cargo test -p fsqlite-core --lib prepared_direct_delete -- --nocapture`
  - `cargo test -p fsqlite-core --lib pending_direct_delete -- --nocapture`
  - `cargo check -p fsqlite-core --lib`
  - `cargo clippy -p fsqlite-core --lib -- -D warnings`
- Fresh-eyes fixes made before benchmark rejection:
  - Removed the `concurrent_txn` eligibility rejection because plain `BEGIN`
    promotes to concurrent mode by default in this project.
  - Registered the conservative `:memory:` concurrent write root before
    mutating the MemDatabase mirror.
  - Forced existing physical delete leaf-run tests through profiling mode so
    they still cover the older physical buffering path.
- Benchmark command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-logical-delete-perf-target cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 250 delete compare isolated`
- Result: rejected. The candidate measured fsqlite delete time `3259ms`
  versus SQLite delete time `37ms`, or `87.39x` slower on the focused isolated
  delete workload.
- Do not retry this logical rowid-message DELETE operator as a standalone
  optimization. The first exact-MemDB hydration plus deferred physical flush
  cost overwhelms the intended per-row ceremony savings.
