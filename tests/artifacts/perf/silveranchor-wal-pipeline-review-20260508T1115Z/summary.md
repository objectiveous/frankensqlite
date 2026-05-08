# Source-Locked Frontier Review

Date: 2026-05-08 11:15Z
Agent: SilverAnchor
Source HEAD: `58b59d9bcf1fa126cce6a167001f6630ba895e03`

## Scope

This pass followed the current full-frontier snapshot after
`tests/artifacts/perf/boldlion-setup-mutation-review-20260508T1040Z/summary.md`.
The live matrix still points at three remaining slower families:

- 100-row prepared direct UPDATE/DELETE tails.
- Small fixed-cost direct INSERT tails.
- The low-thread concurrent writer row.

The first two families route through the dirty shared-worktree DML candidate in:

- `crates/fsqlite-core/src/connection.rs`
- `crates/fsqlite-btree/src/cursor.rs`

Agent Mail showed those files were exclusively reserved by WindyIbis until
`2026-05-08T12:36:04Z`, so this pass did not edit, stage, or revert them.

## Dirty DELETE Leaf-Run Smoke

The dirty candidate appears to implement a pending direct DELETE leaf-run:

- connection-level `PendingDirectDeleteLeafRun` buffering,
- `BtCursor::current_table_leaf_rowid_bounds`,
- `BtCursor::table_delete_current_leaf_rowids_no_rebalance`,
- flush/clear boundaries on read, commit, rollback, savepoint, release, and DDL.

Read-only package compile passed:

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-leafrun-check CARGO_BUILD_JOBS=12 cargo check -p fsqlite-btree -p fsqlite-core --lib
```

The `rch` wrapper was terminated only after the compile success was visible,
because post-build target retrieval was mirroring a very large target tree.

The focused dirty-candidate smoke regressed the exact 100-row isolated DELETE
kernel:

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-leafrun-run CARGO_BUILD_JOBS=12 cargo run --quiet -p fsqlite-e2e --bin perf-update-delete --profile release-perf -- 100 20000 delete fsqlite isolated
```

Output:

```text
perf-update-delete: rows=100 iters=20000 which=delete engine=fsqlite mode=isolated (do_update=false do_delete=true update_count=10 delete_count=5)
  (first isolated delete iter complete)
fsqlite: total=350ms populate=28ms update=0ms delete=320ms  |  per-row-update=0ns  per-row-delete=3203ns
```

The current clean baseline in
`../boldlion-setup-mutation-review-20260508T1040Z/summary.md` was
`1754ns/delete` for the same `100 20000 delete fsqlite isolated` family.
This dirty shape is therefore about `1.83x` slower than the current clean
kernel baseline before any broader focused DML or full-quick gate.

## WAL Lane Reservation

The only non-DML source lane that matched the remaining concurrent-writer tail
was WAL/prepared-frame work. A reservation request for the source files was
denied:

```text
crates/fsqlite-wal/src/checksum.rs -> held by WindyIbis until 2026-05-08T11:43:21Z
crates/fsqlite-wal/src/wal.rs      -> held by WindyIbis until 2026-05-08T11:43:21Z
```

The current WAL source lane also already has a fresh rejected probe in
`../windyibis-wal-pipeline-precompute-20260508T102216Z/summary.md`: its
standalone `mt-mvcc-bench` row improved, but
`comprehensive-bench --quick --filter concurrent` worsened. This pass therefore
did not try to re-enter WAL source while the files were locked.

## Decision

No source change was safe to keep from this pass.

The current dirty DELETE leaf-run shape should not be landed as-is: it compiles
but loses the isolated 100-row DELETE smoke by a large margin. A revised DML
candidate needs to prove the isolated DELETE kernel first, then the focused
UPDATE/DELETE section, before spending a full quick run.

The WAL lane remains a possible future target only as a broader pipeline change
that beats the clean concurrent quick gate, not as the already-rejected
checksum-transform precompute probe.
