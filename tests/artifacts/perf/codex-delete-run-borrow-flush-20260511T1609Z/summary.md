# Delete Run Borrowed Flush Perf Proof

Date: 2026-05-11

## Change

Avoid cloning every pending direct-delete leaf run before flush. The success path
now takes pending runs by value, flushes them in place, and restores the original
buffers only on error.

## Baseline

- Artifact: `tests/artifacts/perf/codex-memory-direct-pageio-candidate-20260511T1255Z/full-quick-noprofile.json`
- Commit recorded by artifact: `dcf1a207b2b563a6880b18a512b6e19b607d35e5`
- Primary score: `0.3679685474039548`
- Write-single average ratio: `1.3326511651276567`
- UPDATE/DELETE average ratio: `1.5914222636123088`

## Candidate

- Artifact: `tests/artifacts/perf/codex-delete-run-borrow-flush-20260511T1609Z/full-quick-final-local.json`
- Commit recorded by artifact: `77bb36a0dbbbae52d4b59da26ebe8fac4d207653`
- Primary score: `0.35579641400391177`
- Write-single average ratio: `1.271451772537187`
- UPDATE/DELETE average ratio: `1.5018117933340127`
- Scenarios: `93`
- FrankenSQLite faster: `80`
- C SQLite faster: `9`

Lower ratios and lower primary score are better.

## Command

```bash
/data/tmp/codex-delete-run-borrow-flush-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/codex-delete-run-borrow-flush-20260511T1609Z/full-quick-final-local.json \
  --no-html \
  > tests/artifacts/perf/codex-delete-run-borrow-flush-20260511T1609Z/full-quick-final-local.stdout
```

## Verification

```bash
env CARGO_TARGET_DIR=/data/tmp/codex-delete-run-borrow-flush-target CARGO_BUILD_JOBS=8 cargo check --workspace --all-targets
env CARGO_TARGET_DIR=/data/tmp/codex-delete-run-borrow-flush-target CARGO_BUILD_JOBS=8 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
env CARGO_TARGET_DIR=/data/tmp/codex-delete-run-borrow-flush-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-btree table_leaf_delete_run -- --nocapture --test-threads=1
env CARGO_TARGET_DIR=/data/tmp/codex-delete-run-borrow-flush-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core pending_direct_delete_leaf_run -- --nocapture --test-threads=1
```

`ubs crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs`
was run and exited non-zero from existing file-wide inventories in these large
files. The changed hunks add no unwraps, panics, SQL construction, casts, or
unchecked indexing.
