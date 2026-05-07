# Compact Prepared Concat Segments

Date: 2026-05-07
Agent: CrimsonGorge

## Target

Prepared direct-simple INSERT record construction was still spending a large
row-build share on concat-heavy benchmark values such as:

- `('user_' || ?1)`
- mixed literal/placeholder text chains in `medium_6col`
- wider `large_10col` rows with long text fields

The retained change compiles top-level `||` chains into compact concat
segments so text literals, NULL literals, placeholders, and fallback expressions
can be handled without repeatedly matching full prepared-expression variants in
the per-row encoder.

## Evidence

Baseline artifacts:

- `full-current.json`
- `insert-profile-current.json`
- `record-profile.json`

Candidate artifacts:

- `compact-concat-insert.json`
- `compact-concat-full.json`
- `local-baseline-full.json`

Focused INSERT profile:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| average ratio | 0.9805944346 | 0.9272610277 |
| p90 ratio | 1.2834522575 | 1.1514490705 |
| C SQLite faster rows | 9 | 6 |

Full quick matrix:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| average ratio | 0.5098531661 | 0.4786135613 |
| geomean ratio | 0.2849970950 | 0.2744489407 |
| p90 ratio | 1.1730124771 | 1.0729948193 |
| p99 ratio | 2.0173983616 | 1.5780785461 |
| C SQLite faster rows | 14 | 11 |
| FrankenSQLite faster rows | 74 | 78 |

Local same-machine baseline check:

| Metric | Local baseline | Candidate |
| --- | ---: | ---: |
| average ratio | 0.5110643564 | 0.4786135613 |
| geomean ratio | 0.2850611777 | 0.2744489407 |
| p90 ratio | 1.1466568993 | 1.0729948193 |
| p99 ratio | 2.3054165143 | 1.5780785461 |
| C SQLite faster rows | 16 | 11 |

The local baseline had noisy large-row outliers, so the strongest causal proof
is the row-build profile drop rather than the largest full-matrix deltas:

| Scenario | Baseline row_build_ns | Candidate row_build_ns |
| --- | ---: | ---: |
| `fs_insert_single_txn_medium_6col_1000` | 247190 | 198958 |
| `fs_insert_single_txn_medium_6col_10000` | 2582524 | 1967020 |
| `fs_insert_single_txn_large_10col_100` | 67558 | 40704 |
| `fs_insert_single_txn_large_10col_1000` | 624909 | 410975 |
| `fs_insert_single_txn_large_10col_10000` | 5787281 | 4165896 |
| `fs_insert_record_size_large_10col_10000` | 6015812 | 4095056 |

## Isomorphism Proof

- Ordering preserved: yes. The concat collector still recursively visits left
  then right, matching the original flattened chain order.
- NULL behavior preserved: yes. A NULL segment marks the result NULL and the
  scratch range is discarded, matching SQLite `||` behavior and the previous
  direct encoder.
- Later bind errors preserved: yes. Placeholder and expression segments are
  still evaluated after a prior NULL; the regression test
  `test_prepared_direct_simple_insert_concat_chain_checks_later_bind_errors_after_null`
  covers this.
- Fallback expressions preserved: yes. Non-literal/non-placeholder concat
  operands become boxed prepared expressions and are evaluated through the
  existing evaluator.
- Floating-point: unchanged. Numeric literals and fallback expressions continue
  through the existing `SqliteValue` text conversion path.
- RNG seeds: N/A.

## Verification Commands

Already completed for the candidate:

```bash
env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-compact-concat-test-target \
  CARGO_BUILD_JOBS=16 \
  cargo test -p fsqlite-core test_prepared_insert_ -- --nocapture

env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-compact-concat-local-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf

FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-compact-concat-local-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/large-record-gap-crimsongorge-20260507T1610Z/compact-concat-insert.json \
  --no-html

/data/tmp/frankensqlite-compact-concat-local-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/large-record-gap-crimsongorge-20260507T1610Z/compact-concat-full.json \
  --no-html
```

Workspace verification was run after this artifact was written; see the session
closeout for the final status.
