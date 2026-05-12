# Concurrent Early Held-Page Abort Probe

- Date: 2026-05-12
- Source baseline: current `main` at `3e67ca93da5eb550c74dafc0a97467acfe05483c`.
- Candidate: in `crates/fsqlite-vdbe/src/engine.rs`, return `BusySnapshot`
  immediately when a first-touch MVCC page write finds an active holder for
  the same page, instead of parking until holder release and then retrying the
  lock acquisition path.
- Candidate status: rejected and manually unwound; no source change kept.

## Commands

```bash
cargo fmt --check
env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-next-target CARGO_BUILD_JOBS=1 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
FSQLITE_BENCH_PROFILE_CONCURRENT=1 /tmp/frankensqlite-codex-next-target/release-perf/comprehensive-bench --quick --filter concurrent --no-html --json-out /tmp/frankensqlite-candidate-concurrent-early-abort.json
```

The candidate build and `cargo fmt --check` passed before measurement.

## Evidence

| Row | baseline F ms | candidate F ms | baseline ratio | candidate ratio | baseline waits | candidate waits | baseline stale | candidate stale | baseline plan errors | candidate plan errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 14.722367 | 13.673415 | 1.209351 | 1.195545 | 12 | 0 | 12 | 39 | 0 | 0 |
| 4 writers x 1000 rows | 23.577873 | 42.872050 | 1.225243 | 2.198877 | 81 | 81 | 72 | 210 | 7 | 72 |
| 8 writers x 1000 rows | 45.963463 | 125.994202 | 0.506065 | 1.392770 | 406 | 234 | 322 | 693 | 21 | 116 |

## Readout

The candidate removed the explicit write-body busy-retry counter on the
2-writer row and gave a small local 2-writer time improvement, but it converted
active-holder waits into many more whole-transaction restarts. The 4-writer row
regressed by roughly 82% in FrankenSQLite median time and the 8-writer row lost
the existing scale-out win entirely. This confirms the holder wait is serving
as useful admission control under higher contention; preempting it is not a
safe standalone policy.

Do not retry active-holder early `BusySnapshot` or immediate page-lock
preemption as a standalone low-thread concurrent optimization. Reconsider only
inside a broader page-builder/MVCC-publication representation change that can
avoid rebuilding the whole transaction after the preemption.
