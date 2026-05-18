# Direct Memory Write-Set Resync Candidate - Rejected

- Date: 2026-05-18
- Candidate scratch checkout:
  `/data/tmp/frankensqlite-write-set-sync-scratch-20260518b`
- Baseline scratch checkout:
  `/data/tmp/frankensqlite-write-set-sync-scratch-20260518`
- Candidate: keep the existing direct-write sync call sites, but remove the
  root-memo early return from `sync_memory_concurrent_pending_write_pages` so
  each direct `:memory:` write resyncs the active transaction's conservative
  page set into the concurrent handle with prepared write markers.
- Correctness checks in the candidate scratch:
  - `cargo fmt --check`
  - `cargo check -p fsqlite-core --lib`
  - `cargo test -p fsqlite-core --lib test_prepared_direct_simple_insert_executes_inside_explicit_transaction -- --nocapture`
  - `cargo test -p fsqlite-core --lib test_prepared_direct_simple_insert_resyncs_after_savepoint_rollback -- --nocapture`
  - `cargo clippy -p fsqlite-core --lib -- -D warnings`
- Candidate benchmark command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-write-set-sync-bench-target-20260518b cargo run --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- --rows-per-thread=100 --threads=16 --iters=1 --json-output=tests/artifacts/perf/codex-write-set-sync-candidate-20260518b.json --summary-md=tests/artifacts/perf/codex-write-set-sync-candidate-20260518b.md`
- Candidate result:
  16-thread shared table, 100 rows/thread, fsqlite 60,710 writes/sec,
  SQLite 8,820 writes/sec, throughput ratio 6.88x, fsqlite failed rows 0,
  SQLite failed rows 0.
- Baseline benchmark command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-write-set-sync-baseline-target-20260518 cargo run --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- --rows-per-thread=100 --threads=16 --iters=1 --json-output=tests/artifacts/perf/codex-write-set-sync-baseline-20260518.json --summary-md=tests/artifacts/perf/codex-write-set-sync-baseline-20260518.md`
- Baseline result:
  16-thread shared table, 100 rows/thread, fsqlite 90,327 writes/sec,
  SQLite 4,824 writes/sec, throughput ratio 18.72x, fsqlite failed rows 0,
  SQLite failed rows 0.
- Decision: rejected. This reduced-row smoke does not reproduce the
  BUSY_SNAPSHOT storm, and the candidate loses roughly one third of baseline
  fsqlite throughput.
