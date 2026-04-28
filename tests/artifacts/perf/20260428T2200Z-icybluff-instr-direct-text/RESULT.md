# instr Direct Text Borrow Perf Proof

Date: 2026-04-28
Agent: IcyBluff
Baseline function code: parent of this commit
Candidate code: this commit

## Scenario

`instr(X, Y)` on already-text arguments. The prior text path converted both
arguments through `SqliteValue::to_text()` before checking empty cases and doing
the character-position search. For `SmallText` inputs, those conversions allocate
owned strings even though the search only needs borrowed `&str` views.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
| --- | ---: | ---: | ---: | ---: |
| `InstrFunc::invoke` text argument conversion | 4 | 5 | 1 | 20 |

## Candidate

Use the existing `text_arg()` helper for the non-blob `instr` path. Text values
borrow their existing `SmallText` contents; blobs and numeric values still use
the existing text conversion semantics through the helper. The blob/blob fast
path remains unchanged.

## Isomorphism

- Ordering preserved: yes. The function still checks `NULL`, then blob/blob,
  then text search.
- Tie-breaking unchanged: yes. `str::find` still returns the first byte match.
- Character position unchanged: yes. The byte match is still converted with
  `haystack[..byte_pos].chars().count() + 1`.
- Floating-point: N/A; non-text values still flow through `to_text()`.
- NULL behavior: unchanged. `null_propagate(args)` still returns `NULL` before
  conversion.
- Empty needle/haystack behavior: unchanged.

## Benchmark Command

Baseline:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-instr-baseline cargo test -p fsqlite-func perf_instr_text_args -- --ignored --nocapture
```

Candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-instr-candidate cargo test -p fsqlite-func perf_instr_text_args -- --ignored --nocapture
```

Raw output: `baseline.txt`, `candidate.txt`.

## Results

| Benchmark | Baseline best | Candidate best | Delta |
| --- | ---: | ---: | ---: |
| `perf_instr_text_args` | 9,275,329 ns | 5,129,616 ns | -44.696% |

Workload: 100,000 invocations, 2 text arguments, 5 repeats. Result stayed at
character position 17.

## Verification

```bash
rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs
git diff --check -- crates/fsqlite-func/src/builtins.rs
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-instr-verify cargo test -p fsqlite-func instr -- --nocapture
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-instr-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-instr-verify cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260428T2200Z-icybluff-instr-direct-text/RESULT.md
```

All verification commands passed. UBS exited 0 with 0 critical issues; it also
reported the existing warning inventory for `builtins.rs`.
