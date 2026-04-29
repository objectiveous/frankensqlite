# Unicode Text Argument Borrowing

## Scenario

- Run ID: `20260429T0035Z-icybluff-unicode-borrow-text`
- Base revision: `5d020155`
- Workload: ignored unit benchmark `cargo test -p fsqlite-func perf_unicode_text_arg -- --ignored --nocapture`
- Iterations: 1,000,000 invocations, best of 7 repeats
- Target: `unicode(X)` with a text input
- Build/test shape: `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unicode-* cargo ...`
- Toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score | Evidence |
|---|---:|---:|---:|---:|---|
| `UnicodeFunc` clones text with `to_text()` before reading only the first char | 4 | 5 | 1 | 20.0 | `baseline.txt`, `candidate.txt` |

The optimization keeps the existing Unicode result calculation and only changes text argument access from eager `String` allocation to borrowed `Cow<str>` when the value is already text.

## Results

| Case | Baseline best ns | Candidate best ns | Change |
|---|---:|---:|---:|
| `unicode("Alphabet soup")` | 39,206,614 | 13,236,117 | -66.240% |

Both runs reported `checksum=455000000`.

Raw output:

- `baseline.txt`
- `candidate.txt`

## Isomorphism Proof

- Ordering preserved: yes. The same first-character lookup is performed on the same text view.
- Tie-breaking unchanged: N/A.
- Floating-point: N/A.
- RNG seeds: N/A.
- NULL behavior: unchanged. `NULL` still returns `NULL` before conversion.
- Type coercion behavior: unchanged for non-text values. `text_arg` falls back to `SqliteValue::to_text()` when a borrowed text view is unavailable.
- Empty string behavior: unchanged. No first character still returns `NULL`.

## Verification

- `rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs`
- `git diff --check -- crates/fsqlite-func/src/builtins.rs`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unicode-verify cargo test -p fsqlite-func unicode -- --nocapture`
- `cargo fmt --check`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unicode-verify cargo check --workspace --all-targets`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unicode-verify cargo clippy --workspace --all-targets -- -D warnings`
- `TMPDIR=/data/tmp ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260429T0035Z-icybluff-unicode-borrow-text/RESULT.md`
