# concat Direct Text Append Perf Proof

Date: 2026-04-28
Agent: IcyBluff
Baseline source commit: bbdb93f1 perf(func): publish concat_ws direct append proof
Candidate code: this commit

## Candidate

Replace per-argument `arg.to_text()` in `concat` with direct appends through
`text_arg(arg).as_ref()`. Text arguments borrow their existing `SmallText`
contents instead of allocating one `String` per argument. `NULL` remains skipped
as an empty argument, matching the existing `concat` behavior.

## Benchmark Command

Baseline:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-baseline cargo test -p fsqlite-func perf_concat_text_args -- --ignored --nocapture
```

Candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-candidate cargo test -p fsqlite-func perf_concat_text_args -- --ignored --nocapture
```

Raw output: `baseline.txt`, `candidate.txt`.

## Results

| Benchmark | Baseline best | Candidate best | Delta |
| --- | ---: | ---: | ---: |
| `perf_concat_text_args` | 50,571,320 ns | 20,362,527 ns | -59.735% |

Workload: 50,000 invocations, 24 text arguments, 5 repeats. Output length stayed
168 bytes.

## Verification

```bash
cargo fmt --check
git diff --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-verify cargo test -p fsqlite-func concat -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-verify cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260428T2110Z-icybluff-concat-direct-text/RESULT.md
```

All verification commands passed. UBS exited 0 with 0 critical issues; it also
reported the existing warning inventory for `builtins.rs`.
