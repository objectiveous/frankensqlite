# concat_ws Direct Append Perf Proof

Date: 2026-04-28
Agent: IcyBluff
Baseline source commit: 17cfb2c6 perf(core): publish GF256 static table proof
Benchmark harness commit: ab9caeb7 perf(func): add ignored concat_ws benchmark for text-arg hot path
Candidate code commit: 1ab3e6bb perf(func): concat_ws builds output in-place, avoiding intermediate Vec
Verification HEAD: 1ab3e6bb perf(func): concat_ws builds output in-place, avoiding intermediate Vec

## Candidate

Replace `concat_ws` intermediate `Vec<String>` collection plus `join` with
direct appends into the result string. Text arguments use the existing borrowed
`text_arg` path, so the common all-text case avoids per-argument string
allocation while preserving SQLite NULL behavior.

## Benchmark Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-ws-baseline cargo test -p fsqlite-func perf_concat_ws_text_args -- --ignored --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-ws-candidate cargo test -p fsqlite-func perf_concat_ws_text_args -- --ignored --nocapture
```

Raw output:

- `baseline.txt`
- `candidate.txt`

## Results

| Benchmark | Baseline best | Candidate best | Delta |
| --- | ---: | ---: | ---: |
| `perf_concat_ws_text_args` | 74,927,764 ns | 24,453,767 ns | -67.364% |

Workload: 50,000 invocations, 24 text args, 5 repeats. Output length stayed
191 bytes.

## Rejected Variant

A pre-sizing pass over the arguments was tested and rejected. It measured
34,885,096 ns, slower than the direct-append version because the extra scan cost
outweighed saved growth for this argument shape.

## Verification

```bash
cargo fmt --check
git diff --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-ws-verify cargo test -p fsqlite-func concat_ws -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-ws-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-concat-ws-verify cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260428T2100Z-icybluff-concat-ws-direct/RESULT.md
```

All commands passed. UBS exited 0 with warnings only and no critical findings.
