# Direct INSERT fixed cell array probe - rejected

Run time: 2026-05-08T00:31Z-00:34Z

Git baseline: `5818bfe6 docs(perf): reject cell slot pre-evict probe`

Candidate scratch worktree:
`/data/tmp/frankensqlite-direct-cell-array-tanbear-20260508T0025Z`

Candidate diff:
`candidate-direct-cell-array.diff`

## Hypothesis

`Connection::try_serialize_prepared_direct_simple_insert_record` used
`SmallVec<[PreparedDirectInsertRecordCell; 16]>` as a staging buffer. The probe
replaced that with a fixed `[PreparedDirectInsertRecordCell; 16]` plus a count,
falling back to generic dispatch when the prepared direct INSERT had more than
16 columns.

Expected win: remove `SmallVec` push/container overhead for the benchmarked
direct INSERT row-builder path.

## Correctness/build proof

In the scratch worktree:

- `cargo fmt -p fsqlite-core --check` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-cell-array-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-cell-array-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf` passed.

## Benchmark gate

Direct binaries were run in alternating same-window pairs:

- Baseline: `/data/tmp/frankensqlite-smallvec-isolated-target/release-perf/comprehensive-bench`
- Candidate: `/data/tmp/frankensqlite-direct-cell-array-target/release-perf/comprehensive-bench`

Primary fields are from `.summary`: `score / avg / geomean / p90 / p99 /
csqlite_faster / franken_faster`.

| Run | Baseline | Candidate | Verdict |
| --- | --- | --- | --- |
| insert 1 | `0.778579 / 0.863316 / 0.835643 / 1.177931 / 1.293853 / 7 / 16` | `0.818908 / 0.820626 / 0.795466 / 1.129829 / 1.272428 / 5 / 19` | weighted score worse |
| insert 2 | `0.809666 / 0.826215 / 0.787895 / 1.139134 / 1.601360 / 6 / 18` | `0.780930 / 0.815085 / 0.791814 / 1.113496 / 1.146828 / 7 / 18` | mixed |
| insert 3 | `0.792200 / 0.849206 / 0.822069 / 1.157943 / 1.285590 / 8 / 17` | `0.799949 / 0.798619 / 0.775910 / 1.090336 / 1.134319 / 5 / 19` | weighted score worse |
| update 1 | `1.071878 / 1.092029 / 1.071878 / 1.420417 / 1.420417 / 2 / 2` | `1.131122 / 1.146445 / 1.131122 / 1.426859 / 1.426859 / 3 / 0` | worse |
| update 2 | `1.152278 / 1.170002 / 1.152278 / 1.481543 / 1.481543 / 3 / 0` | `1.148687 / 1.167197 / 1.148687 / 1.495642 / 1.495642 / 4 / 0` | noise-level mixed |
| update 3 | `1.143582 / 1.163802 / 1.143582 / 1.636787 / 1.636787 / 3 / 1` | `1.143086 / 1.159855 / 1.143086 / 1.475946 / 1.475946 / 4 / 1` | noise-level mixed |

## Result

Rejected. The candidate improved many raw FSQLite INSERT medians and most
non-weighted INSERT aggregate ratios, but the primary weighted INSERT score did
not reliably improve across three runs. The UPDATE section was flat to slightly
worse versus C SQLite and increased C-faster rows in the repeat gate.

The fixed-array shape is not a keepable standalone optimization. It is worth
reconsidering only as part of a broader prepared row-template design that
removes row construction and DML setup costs together, preserves the direct
fast path for more-than-16-column INSERTs, and improves both focused INSERT and
UPDATE/delete sections in the same A/B window.
