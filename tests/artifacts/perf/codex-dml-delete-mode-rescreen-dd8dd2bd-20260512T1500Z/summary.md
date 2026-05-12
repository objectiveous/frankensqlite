# DML Delete Mode Rescreen

- Date: 2026-05-12 15:00 UTC
- Source commit before the helper fix: `dd8dd2bd6bb9fd4581048808a36d7a0a767ca198`
- Current-source build command:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-dml-rescreen-dd8dd2bd CARGO_BUILD_JOBS=4 cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete`
- Note: `rch` could not normalize the `/data/tmp` worktree under `/data/projects` and fell back to a local build.
- Binary used for the valid runs:
  `/data/tmp/frankensqlite-target-dml-rescreen-dd8dd2bd/release-perf/perf-update-delete`

## Commands

- Standard:
  `/data/tmp/frankensqlite-target-dml-rescreen-dd8dd2bd/release-perf/perf-update-delete 10000 200 delete compare standard`
- Isolated:
  `/data/tmp/frankensqlite-target-dml-rescreen-dd8dd2bd/release-perf/perf-update-delete 10000 200 delete compare isolated`
- Sparse isolated:
  `/data/tmp/frankensqlite-target-dml-rescreen-dd8dd2bd/release-perf/perf-update-delete 10000 200 delete compare sparse-isolated`

## Results

| Mode | FrankenSQLite delete ns/row | C SQLite delete ns/row | F/C delete ratio |
| --- | ---: | ---: | ---: |
| `standard` | `476` | `332` | `1.43x` |
| `isolated` | `374` | `285` | `1.31x` |
| `sparse-isolated` | `1123` | `463` | `2.43x` |

## Reading

This rescreen does not open a safe standalone source lever.

- `standard` still shows a direct DELETE gap, even though total runtime is green because FrankenSQLite's populate phase is faster.
- `isolated` keeps all deletes in one transaction and still reports `1.31x` F/C on delete mutation, so a pure transaction-envelope trim is not enough.
- `sparse-isolated` preserves the benchmark's every-20th-row sparse shape across one large transaction and worsens to `2.43x` F/C, pointing at row-existence/seek/read-view work rather than only commit ceremony.

The comparison against C SQLite's path is consistent with the previous focused DML artifact. C SQLite reuses a prepared cursor through `OP_SeekRowid` and `OP_Delete`, then calls `sqlite3BtreeDelete` on the positioned cursor. FrankenSQLite's direct path is already fully direct and batches same-leaf physical publication, but each statement must still return an immediate affected-row count and preserve read-your-writes semantics. Skipping the row-existence proof would require a transaction-local mutation/read-view overlay, not just another pending leaf-run tweak.

## Decision

No DELETE source optimization from this rescreen. Keep the existing boundary from `tests/artifacts/perf/codex-dml-current-profile-after-mtfix-20260512T1230Z/`: reconsider DML source work only as the broader transaction-local DML mutation/read-view operator, with correctness proof for affected rows, read-your-writes, rollback/savepoints, duplicate rowids, reads between writes, and focused/fullquick benchmark wins.

One unrelated helper bug was found during this pass: `perf-update-delete --help` followed the positional parser and failed as an invalid row count despite the binary documenting usage. That is fixed in `crates/fsqlite-e2e/src/bin/perf_update_delete.rs`.
