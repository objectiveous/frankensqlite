# Retained Direct Fixed REAL UPDATE Run Rejection

- Date: 2026-05-07T12:37:41Z
- Baseline HEAD: `5b36871d docs(perf): reject staged table leaf delete`
- Candidate worktree: `/data/tmp/frankensqlite-candidate-retained-direct-real-update-20260507T123741Z`
- Baseline worktree: `/data/tmp/frankensqlite-baseline-retained-direct-real-update-20260507T123741Z`
- Candidate patch: `crates/fsqlite-core/src/connection.rs` only

## Candidate

Buffered monotone explicit-transaction direct UPDATEs of a single fixed-width
REAL column and flushed the run with one retained B-tree cursor at read,
commit, savepoint, release, DDL, and table-program boundaries.

The focused proof passed in the candidate source:

```text
env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_fixed_real_update_run_flushes_on_read_and_commit -- --nocapture
```

Existing direct UPDATE/DELETE guards also passed:

```text
env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture
env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture
env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_fast_path_update_delete_ddl_invalidation -- --nocapture
```

## Isolated Matrix

Rows are `perf-update-delete` isolated per-row timings.

| Workload | Baseline FSQLite | Candidate FSQLite | Result |
|---|---:|---:|---|
| update 100 | 656 ns | 642 ns | noise-level win |
| update 1000 | 869 ns | 838 ns | win in this run, but earlier same-window probe was worse |
| update 10000 | 888 ns | 910 ns | reject |
| delete 100 | 1123 ns | 1247 ns | reject |
| delete 1000 | 1176 ns | 1247 ns | reject |
| delete 10000 | 1254 ns | 1289 ns | reject |

Saved raw command output:

- `baseline-update-100.txt`
- `baseline-update-1000.txt`
- `baseline-update-10000.txt`
- `candidate-update-100.txt`
- `candidate-update-1000.txt`
- `candidate-update-10000.txt`
- `baseline-delete-100.txt`
- `baseline-delete-1000.txt`
- `baseline-delete-10000.txt`
- `candidate-delete-100.txt`
- `candidate-delete-1000.txt`
- `candidate-delete-10000.txt`

## Decision

Rejected. The retained fixed-REAL update run is correct in focused tests but
does not clear the isolated keep gate. It adds buffering and read-boundary
flush complexity while the large-row update target regresses and delete rows
show collateral noise/regression. Do not retry this exact connection-only
buffering shape unless the benchmark can prove the real update workload keeps
the exact row mirror hot for long monotone runs and the same-window isolated
matrix improves update 1000/10000 without delete regressions.
