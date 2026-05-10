# Logical DELETE Tombstone Probe - 2026-05-10

## Scope

Prototype branch/worktree:

- Branch: `codex/logical-tombstone-probe-20260510`
- Worktree: `/data/tmp/frankensqlite-codex-logical-tombstone-probe-20260510`
- Base: `9b0fe46b Record DELETE full quick rejection`

The probe added a private `:memory:` direct-DELETE tombstone buffer in
`crates/fsqlite-core/src/connection.rs`. Prepared direct rowid DELETEs inside
explicit transactions record logical rowid tombstones instead of immediately
calling `BtCursor::delete`; reads/savepoints/incompatible writes materialize the
tombstones.

## Correctness Gate

Command:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-logical-tombstone-target cargo test -p fsqlite-core test_memory_prepared_direct_delete_tombstones -- --nocapture
```

`rch` could not offload the `/data/tmp` worktree and ran locally.

Result:

- `test_memory_prepared_direct_delete_tombstones_defer_until_read_after_commit` passed
- `test_memory_prepared_direct_delete_tombstones_repeat_and_rollback` passed

## Focused Performance Gate

Build:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-logical-tombstone-release-target cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Focused compare runs:

| Mode | Rows | Iters | F per-row DELETE | C per-row DELETE | F/C |
|---|---:|---:|---:|---:|---:|
| standard | 100 | 10 | 1783 ns | 465 ns | 3.84x |
| standard | 1000 | 5 | 729 ns | 364 ns | 2.00x |
| standard | 10000 | 3 | 709 ns | 355 ns | 2.00x |
| isolated | 10000 | 3 | 597 ns | 278 ns | 2.15x |

## Decision

Reject as a standalone keep. The logical tombstone buffer was correct for the
narrow private `:memory:` envelope, but it did not improve the focused DELETE
tail enough to justify a full quick matrix run. The residual cost is still in
rowid seek / direct statement ceremony, not physical page deletion or leaf
compaction.

Worth retrying only if paired with a measured seek-elision design, such as a
monotonic rowid delete cursor that advances within retained leaf state without a
fresh root descent per row.
