# sqlite_compileoption_used Text Argument Borrowing

## Scenario

- Run ID: `20260429T0155Z-icybluff-compileoption-borrow-text`
- Parent revision: `62c37ae7` (`perf(func): publish UNHEX borrowed text proof`)
- Code revision: this commit
- Workload: `cargo test -p fsqlite-func perf_compileoption_used_text_args -- --ignored --nocapture`
- Iterations: 1,000,000 calls per case, best of 7 repeats
- Target: `sqlite_compileoption_used(X)` with borrowed text input
- Toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

The optimization replaces owned `String` conversion for text arguments with the
existing `text_arg` borrowing helper before matching against the static compile
option list. Non-text arguments still allocate through `to_text` via `text_arg`.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
|---|---:|---:|---:|---:|
| `SqliteCompileoptionUsedFunc::invoke` text conversion | 3 | 5 | 1 | 15.0 |

## Results

| Case | Baseline best ns | Candidate best ns | Delta | Checksum |
|---|---:|---:|---:|---:|
| `sqlite_compileoption_used("SQLITE_ENABLE_ICU")` | 117,734,555 | 91,442,789 | -22.331% | 7,000,000 |
| `sqlite_compileoption_used("ENABLE_NOT_PRESENT")` | 159,447,240 | 131,355,042 | -17.618% | 7,000,000 |

Artifacts:

- `baseline.txt`
- `candidate.txt`

## Isomorphism

- Ordering preserved: yes. The same static compile-option table is scanned in the same order.
- Prefix and case folding rules: unchanged. Matching still flows through `sqlite_compileoption_used`.
- Floating point: not applicable.
- RNG seeds: not applicable.
- NULL behavior: unchanged. NULL still returns NULL before conversion.
- Non-text fallback: unchanged. `text_arg` still calls `to_text` for non-text values.
- Output equivalence: checksum stayed at `7,000,000`.

## Verification

Commands run for this slice:

```bash
rustfmt --edition 2024 crates/fsqlite-func/src/builtins.rs
TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-compileoption-baseline cargo test -p fsqlite-func perf_compileoption_used_text_args -- --ignored --nocapture
TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-compileoption-candidate cargo test -p fsqlite-func perf_compileoption_used_text_args -- --ignored --nocapture
```

Workspace verification is recorded in the session closeout. Full `cargo fmt
--check` is expected to remain blocked by unrelated pre-existing formatting in
`crates/fsqlite-core/src/connection.rs`.
