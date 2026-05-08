# WAL Prepared Transform Precompute Probe

Date: 2026-05-08
Agent: WindyIbis
Baseline source: `638e93f9` (`main`)
Candidate source: detached scratch worktree
`/data/tmp/frankensqlite-windyibis-wal-pipeline-638e93f9`

## Scope

This probe followed the current low-thread concurrent-writer profile, where
`WalChecksumTransform::for_wal_frame` remained a useful self-time entry. The
candidate was deliberately not the previously rejected header-only transform.
It changed the prepared WAL frame construction path to:

- precompute the header and payload affine checksum coefficients once per
  prepared batch;
- build the header transform from the serialized first 8 frame-header bytes;
- build the payload transform from the original page payload slice instead of
  rereading the freshly copied frame payload.

The candidate touched only these scratch-worktree source files:

- `crates/fsqlite-wal/src/checksum.rs`
- `crates/fsqlite-wal/src/wal.rs`

The shared `main` worktree source was not edited. The candidate diff is saved as
`candidate.diff`.

## Coordination

Agent Mail was degraded during this pass. The initial source reservation macro
timed out, a direct source reservation attempt timed out, and the later
reservation attempts for this artifact plus
`docs/progress/perf-negative-results.md` returned `Resource is temporarily
busy`. The source probe therefore stayed in a detached scratch worktree.

## Correctness

Focused checksum coverage passed on the scratch candidate:

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-wal-pipeline-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-wal checksum_transform -- --nocapture
```

`rch` failed open to local execution because the scratch worktree lives under
`/data/tmp`; the test run passed 3 checksum-transform tests, including the new
precomputed-coefficients equivalence case.

## Focused 2-Thread Result

The standalone `mt-mvcc-bench` row moved in the intended direction in a
same-window A/B:

| Run | FSQLite p50 | C SQLite p50 | Time ratio | Throughput ratio |
| --- | ---: | ---: | ---: | ---: |
| baseline | `3.75 ms` | `2.45 ms` | `1.5307x` | `0.6564x` |
| candidate | `3.35 ms` | `2.53 ms` | `1.3217x` | `0.7567x` |

Files:

- `baseline-mt-mvcc-2t.json`
- `candidate-mt-mvcc-2t.json`

## Concurrent Quick Gate

The broader `comprehensive-bench --quick --filter concurrent` gate rejected the
candidate:

| Scenario | Baseline FSQLite | Candidate FSQLite | Baseline ratio | Candidate ratio |
| --- | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | `13.379 ms` | `13.638 ms` | `1.1312x` | `1.1521x` |
| 4 writers x 1000 rows | `19.410 ms` | `19.734 ms` | `0.9668x` | `1.0081x` |
| 8 writers x 1000 rows | `35.013 ms` | `34.298 ms` | `0.3774x` | `0.3768x` |

Aggregate concurrent score also worsened:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| average ratio | `0.825150` | `0.845684` |
| geomean ratio | `0.744574` | `0.759243` |
| p90/p99 ratio | `1.131202` | `1.152098` |

Files:

- `baseline-concurrent-quick.json`
- `candidate-concurrent-quick.json`

## Decision

Rejected. The scratch candidate improved the standalone 2-thread harness but
lost the actual concurrent quick matrix, including the primary 2-writer row.
No source change was kept or applied to `main`, and no full quick matrix was run
because the focused section gate failed.

Do not retry per-batch WAL checksum coefficient precompute plus source-payload
prepared-transform construction as a standalone optimization. Revisit WAL frame
preparation only with a larger pipeline change that improves
`comprehensive-bench --quick --filter concurrent` and then the full quick
matrix in the same A/B window.
