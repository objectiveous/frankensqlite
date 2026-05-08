# Direct INSERT header-size fast path - rejected

Run time: 2026-05-08T00:45Z-00:55Z

Baseline source: `5818bfe6 docs(perf): reject cell slot pre-evict probe`

Candidate scratch worktree:
`/data/tmp/frankensqlite-direct-header-fastpath-tanbear-20260508T0045Z`

Candidate diff: `direct-header-fastpath.diff`

## Candidate

`Connection::prepared_direct_insert_record_header_size(content_size)` still used
the fixed-point loop even when `content_size <= 126`. In that range, the
initial `header_size = content_size + 1` is at most `127`, so the header-size
varint is exactly one byte and the original loop returns the same value on its
first iteration.

The candidate added:

```rust
if content_size <= 126 {
    return content_size + 1;
}
```

This mirrors the already-landed record-layer header-size fast path, but in the
direct prepared INSERT serializer.

## Correctness/build proof

In the scratch worktree:

- `cargo fmt -p fsqlite-core --check` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-header-fastpath-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-header-fastpath-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf` passed.

## Focused insert gate

Direct binaries:

- Baseline: `/data/tmp/frankensqlite-smallvec-isolated-target/release-perf/comprehensive-bench`
- Candidate: `/data/tmp/frankensqlite-direct-header-fastpath-target/release-perf/comprehensive-bench`

Three alternating `--quick --filter insert` pairs were run. Primary fields are
`.summary`: `score / avg / geomean / p90 / p99 / csqlite_faster /
franken_faster`.

| Run | Baseline | Candidate | Verdict |
| --- | --- | --- | --- |
| 1 | `0.803945 / 0.884571 / 0.836966 / 1.137335 / 1.912852 / 9 / 15` | `0.805853 / 0.858641 / 0.827202 / 1.137439 / 1.515223 / 7 / 18` | weighted score worse |
| 2 | `0.800119 / 0.836474 / 0.811933 / 1.114554 / 1.217112 / 7 / 17` | `0.803174 / 0.806316 / 0.783383 / 1.117836 / 1.214957 / 5 / 19` | weighted score worse |
| 3 | `0.763483 / 0.792955 / 0.766249 / 1.110142 / 1.217314 / 5 / 18` | `0.803002 / 0.834692 / 0.810500 / 1.123786 / 1.148062 / 7 / 17` | worse |

## Result

Rejected. The candidate often improved p99 and some non-weighted INSERT
aggregates, but the primary weighted INSERT score was worse in all three
focused pairs. This is too small and noisy to land as a standalone
connection-layer micro-optimization.

Retry only if the direct serializer is already being refactored into a broader
row-template/page-run writer where this branch is free inside a larger measured
win. Do not retry it as a standalone header-size shortcut.
