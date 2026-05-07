# Prepared param-one integer/float binary INSERT specialization rejected

Date: 2026-05-07 21:12Z
Agent: CrimsonGorge

## Scope

Tested a narrow prepare-time direct INSERT expression specialization in an
isolated scratch worktree:

`/data/tmp/frankensqlite-paramone-intop-crimsongorge-20260507T2112Z`

Source base: `81659ea7920869a6944b47c8b226703b988ba4c7`

The shared checkout was not edited because `crates/fsqlite-core/src/connection.rs`
was peer-reserved for a separate DML investigation.

## Candidate

The candidate added `PreparedDirectSimpleInsertExpr` variants for:

- `?1 <integer-op> integer-literal`, where the op was `+`, `-`, `*`, or `%`
- `?1 <float-op> float-literal`, where the op was `+`, `-`, `*`, or `/`

The intent was to avoid per-row recursive expression walking and temporary
`SqliteValue` construction for simple benchmark row-template columns such as
`?1 * 2` and `?1 % 100`.

Patch artifact:

- `candidate-paramone-intop.diff`

## Commands

Correctness probe:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-paramone-intop-target CARGO_BUILD_JOBS=10 \
  cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture
```

Builds:

```text
env TMPDIR=/data/tmp/frankensqlite-paramone-intop-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-paramone-intop-target \
  CARGO_BUILD_JOBS=10 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

```text
env TMPDIR=/data/tmp/frankensqlite-paramone-intop-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-paramone-baseline-target \
  CARGO_BUILD_JOBS=10 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Focused paired INSERT runs:

```text
/data/tmp/frankensqlite-paramone-baseline-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/paramone-intop-crimsongorge-20260507T2112Z/baseline-insert.json \
  --no-html
```

```text
/data/tmp/frankensqlite-paramone-intop-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/paramone-intop-crimsongorge-20260507T2112Z/candidate-insert.json \
  --no-html
```

Repeat runs used the same commands and wrote `baseline-insert-repeat.json` and
`candidate-insert-repeat.json`.

## Results

First paired run:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 0.8089982083854728 | 0.7984313729504148 |
| Average ratio | 0.8418018208390472 | 0.828166965119695 |
| Geomean ratio | 0.8121668867717653 | 0.7974147178142178 |
| p90 ratio | 1.1329475100942126 | 1.0929989010278622 |
| p99 ratio | 1.2738419349924275 | 1.405180687400819 |
| C SQLite faster rows | 7 | 6 |

Repeat paired run:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 0.7832648833059592 | 0.7755839688181437 |
| Average ratio | 0.7872773218359899 | 0.8593629832333529 |
| Geomean ratio | 0.7596132132504774 | 0.8243556684656996 |
| p90 ratio | 1.0774223876464186 | 1.1127643718037674 |
| p99 ratio | 1.1944813874447535 | 1.7589035182387223 |
| C SQLite faster rows | 4 | 6 |

The primary weighted score moved slightly in the right direction in both pairs,
but the repeat run worsened average/geomean ratios, p90/p99, and C-faster row
count. The worst repeat regression was `100 rows / batched (100/txn)`, whose
ratio moved `1.0468798859272785 -> 1.7589035182387223`.

See `repeat-row-compare.tsv` for the per-row repeat comparison.

## Decision

Rejected and not applied to the shared checkout.

Root cause: the specialization is too narrow to produce a robust step-change.
It removes a small amount of recursive expression ceremony for a subset of
`?1 op literal` templates, but it also adds enum variants and extra hot-path
matching in the already layout-sensitive direct INSERT expression evaluator.
The observed benefit is close enough to code layout and benchmark variance that
the matrix does not justify source churn.

Retry only with a broader row-template VM that precomputes the entire prepared
direct INSERT column program and proves a same-window INSERT geomean and p99
win, not with another single-expression `?1 op literal` micro-specialization.
