# UNISTR Streaming Decode

## Scenario

- Run ID: `20260429T0220Z-icybluff-unistr-stream`
- Parent revision: `c78f0736` (`perf(func): decode UNHEX in one pass`)
- Code revision: this commit
- Workload: `cargo test -p fsqlite-func perf_unistr_text_args -- --ignored --nocapture`
- Iterations: 500,000 calls per case, best of 7 repeats
- Target: `unistr(X)` plain text and escaped text paths
- Toolchain: `rustc 1.97.0-nightly (37d85e592 2026-04-28)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`
- Linker: `RUSTFLAGS='-C linker=cc'` because the active nightly sysroot did not provide `rust-lld`

The optimization replaces owned input conversion and full `Vec<char>` indexing
with borrowed `text_arg` input and streaming escape parsing. Output text is still
materialized once.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
|---|---:|---:|---:|---:|
| `UnistrFunc::invoke` owned text and `Vec<char>` parsing | 4 | 5 | 2 | 10.0 |

## Results

| Case | Baseline best ns | Candidate best ns | Delta | Checksum |
|---|---:|---:|---:|---:|
| `unistr("plain unicode payload")` | 156,162,140 | 59,928,118 | -61.624% | 108,500,000 |
| `unistr("a\\\\b\\u0020\\u0048\\u0069\\U0001f600")` | 218,001,194 | 112,067,879 | -48.593% | 108,500,000 |

Artifacts:

- `baseline.txt`
- `candidate.txt`

## Isomorphism

- Ordering preserved: yes. The input is still scanned left to right.
- Escaped backslash behavior: unchanged. `\\` still produces a single backslash.
- `\uXXXX` behavior: unchanged. Four valid hex digits are decoded into one Unicode scalar value.
- `\UXXXXXXXX` behavior: unchanged. Eight valid hex digits are decoded into one Unicode scalar value.
- Invalid escape behavior: unchanged. Failed escape decoding falls back to a literal backslash and continues.
- Floating point: not applicable.
- RNG seeds: not applicable.
- NULL behavior: unchanged. NULL input still returns NULL before conversion.
- Non-text fallback: unchanged. `text_arg` still calls `to_text` for non-text values.
- Output equivalence: checksum stayed at `108,500,000`.

## Verification

Commands run for this slice:

```bash
rustfmt --edition 2024 crates/fsqlite-func/src/builtins.rs
RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp rch exec -- env RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unistr-baseline-linker cargo test -p fsqlite-func perf_unistr_text_args -- --ignored --nocapture
RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp rch exec -- env RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unistr-candidate-linker cargo test -p fsqlite-func perf_unistr_text_args -- --ignored --nocapture
RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp rch exec -- env RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unistr-verify cargo test -p fsqlite-func unistr -- --nocapture
RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp rch exec -- env RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unistr-isolated cargo check --workspace --all-targets
RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp rch exec -- env RUSTFLAGS='-C linker=cc' TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unistr-isolated cargo clippy --workspace --all-targets -- -D warnings
rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs
cargo fmt --check
TMPDIR=/data/tmp ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260429T0220Z-icybluff-unistr-stream/RESULT.md
```

Workspace `cargo check` and `cargo clippy` passed. Full `cargo fmt --check`
remains blocked by unrelated pre-existing formatting in
`crates/fsqlite-core/src/connection.rs:15030`; touched-file `rustfmt --check`
passed.
