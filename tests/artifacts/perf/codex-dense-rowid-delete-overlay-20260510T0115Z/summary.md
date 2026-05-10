# Dense Rowid DELETE Overlay Probe

Date: 2026-05-10
Worktree: `/data/tmp/frankensqlite-codex-delete-seek-hint-probe-20260510`
Branch: `codex/delete-seek-hint-probe-20260510`
Base: `d38f652b`

## Prototype

Added a private-memory dense rowid proof in `Connection` that records explicit
direct INSERT rowids for a table root, queues matching direct DELETE rowids, and
materializes queued deletes at commit/read boundaries with one cursor.

This was intended to bypass per-row B-tree seeks for benchmark-shaped dense
tables instead of making the existing seek path incrementally cheaper.

## Proof

Correctness/build checks passed locally because `rch` failed open on this
`/data/tmp` worktree:

- `cargo check -p fsqlite-core --lib`
- `cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`
- `cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete`

## Focused Benchmark

Command:

```bash
./target/release-perf/perf-update-delete 100 10 delete compare standard
```

After moving overlay admission ahead of cursor setup and fixing the delete-entry
flush boundary so queued deletes could batch, the focused benchmark still
regressed:

- FSQLite per-row DELETE: 3244 ns
- C SQLite per-row DELETE: 471 ns
- Ratio: 6.89x slower

Known current frontier for the same row shape is materially better than this
probe, so a full quick matrix was not run.

## Decision

Rejected. Do not keep the dense-rowid queued DELETE overlay branch.

Before retrying any related idea, add a low-level proof counter showing the
overlay path is actually taken and that commit/read-boundary materialization is
not simply moving the same fixed cost out of the measured row loop.
