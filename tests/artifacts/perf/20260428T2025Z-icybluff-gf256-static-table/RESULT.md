# GF256 Static Multiply Table Perf Proof

Date: 2026-04-28
Agent: IcyBluff
Baseline commit: 5b8636c0 perf(core): table-drive GF256 symbol addmul
Candidate code commit: 1bcea3e8 perf(core): precompute full GF256 multiplication table at compile time
Verification HEAD: 095f82be fix(pager,core): reject hidden commit headers in single-batch merge; restrict gf256 import

## Candidate

Replace per-call GF(256) multiply row construction with a compile-time
256-by-256 lookup table. The core symbol multiply/addmul paths borrow a static
row for the selected coefficient.

Tradeoff: +64 KiB static read-only data for hot-loop throughput.

## Benchmark Command

```bash
/data/tmp/cargo-target-icybluff-20260428-gf256-static-baseline/release-perf/deps/symbol_ops-c175837d0f8b1a0e --bench 'symbol_ops/symbol_addmul_c53/4096|symbol_ops/symbol_addmul_c53/512|raptorq_paths/decode_fallback_addmul'
/data/tmp/cargo-target-icybluff-20260428-gf256-static-candidate/release-perf/deps/symbol_ops-c175837d0f8b1a0e --bench 'symbol_ops/symbol_addmul_c53/4096|symbol_ops/symbol_addmul_c53/512|raptorq_paths/decode_fallback_addmul'
```

Raw output:

- `baseline-criterion.txt`
- `candidate-criterion.txt`

## Results

| Benchmark | Baseline median | Candidate median | Median delta vs baseline |
| --- | ---: | ---: | ---: |
| `symbol_ops/symbol_addmul_c53/512` | 1.7079 us | 190.26 ns | -88.859% |
| `symbol_ops/symbol_addmul_c53/4096` | 3.1351 us | 1.5154 us | -51.663% |
| `raptorq_paths/decode_fallback_addmul` | 21.449 us | 10.545 us | -50.836% |

Criterion reported `Performance has improved` for all three benchmarks.

## Verification

```bash
cargo fmt --check
git diff --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-gf256-static-verify cargo test -p fsqlite-core gf256 -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-gf256-static-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-gf256-static-verify cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-core/src/lib.rs tests/artifacts/perf/20260428T2025Z-icybluff-gf256-static-table/RESULT.md
```

All commands passed. UBS exited 0 with warnings only.
