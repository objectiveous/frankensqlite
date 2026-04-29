# UNHEX Text Argument Borrowing

## Scenario

- Run ID: `20260429T0140Z-icybluff-unhex-borrow-text`
- Code revision: `ceed021e` (`perf(func): avoid UNHEX text argument allocations`)
- Workload: `cargo test -p fsqlite-func perf_unhex_text_args -- --ignored --nocapture`
- Iterations: 300,000 calls per case, best of 7 repeats
- Target: `unhex(X[,Y])` with text input and optional ignored-character text
- Toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

The optimization replaces owned `String` conversion for UNHEX text arguments with
borrowed `Cow<str>` access via the existing `text_arg` helper. Output bytes are
still materialized exactly as before.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
|---|---:|---:|---:|---:|
| `UnhexFunc::call` text argument conversion | 4 | 5 | 1 | 20.0 |

## Results

| Case | Baseline best ns | Candidate best ns | Delta | Checksum |
|---|---:|---:|---:|---:|
| `unhex("48656C6C6F776F726C64")` | 120,529,021 | 114,120,375 | -5.317% | 31,500,000 |
| `unhex("48-65-6C-6C-6F", "-")` | 101,821,553 | 85,423,877 | -16.103% | 31,500,000 |

Artifacts:

- `baseline.txt`
- `candidate.txt`

## Isomorphism

- Ordering preserved: yes. The filtered input stream is traversed in the same order.
- Tie-breaking unchanged: not applicable.
- Floating point: not applicable.
- RNG seeds: not applicable.
- NULL behavior: unchanged. `text_arg` preserves the existing NULL-to-empty-text path used by the prior `to_text` conversion.
- Non-text fallback: unchanged. Non-text values still flow through `to_text` before parsing.
- Output equivalence: checksum stayed at `31,500,000` for both runs.

## Verification

Commands run for this slice:

```bash
rustfmt --edition 2024 crates/fsqlite-func/src/builtins.rs
TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unhex-baseline cargo test -p fsqlite-func perf_unhex_text_args -- --ignored --nocapture
TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unhex-candidate cargo test -p fsqlite-func perf_unhex_text_args -- --ignored --nocapture
rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs
git diff --check -- crates/fsqlite-func/src/builtins.rs
```

Workspace verification is recorded in the session closeout. At artifact-writing
time, full `cargo fmt --check` was known to fail on unrelated pre-existing
formatting in `crates/fsqlite-core/src/connection.rs`.
