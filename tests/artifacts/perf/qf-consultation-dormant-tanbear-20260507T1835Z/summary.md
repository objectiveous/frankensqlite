# QF consultation dormant candidate remeasure

Date: 2026-05-07 18:35Z
Agent: TanBear

## Scope

Measured the already-dirty `crates/fsqlite-core/src/connection.rs` candidate
that makes quotient-filter consultation explicitly dormant by returning
`Ok(false)` before borrowing the per-table filter map. I did not author or
stage the source diff. The worktree was dirty during measurement.

The candidate overlaps prior no-retry territory around QF maintenance /
consultation, so the keep gate was the focused update/delete matrix before any
source ownership decision.

## Candidate diff shape

- Adds `QUOTIENT_FILTER_CONSULTATION_ENABLED: bool = false`.
- Makes `qf_maybe_short_circuit_for_rowid` return `Ok(false)` immediately when
  disabled.
- Rewrites the focused QF tests to assert that absent rowids still use the
  normal B-tree lookup path and record zero QF short-circuits.

## Correctness and build proof

```text
env TMPDIR=/data/tmp/frankensqlite-qf-dormant-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-qf-dormant-target \
  CARGO_BUILD_JOBS=8 \
  cargo test -p fsqlite-core quotient_filter -- --nocapture
```

Result: passed. The three focused QF tests passed:

- `test_quotient_filter_disabled_delete_then_redelete_uses_btree_lookup`
- `test_quotient_filter_disabled_does_not_short_circuit_absent_rowids_on_delete`
- `test_quotient_filter_rollback_forces_reseed`

```text
env TMPDIR=/data/tmp/frankensqlite-qf-dormant-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-qf-dormant-target \
  CARGO_BUILD_JOBS=8 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Result: passed.

Logs:

- `stdout/qf-tests.log`
- `stdout/build-bench.log`

## Measurement

```text
/data/tmp/frankensqlite-qf-dormant-target/release-perf/comprehensive-bench \
  --quick \
  --filter UPDATE \
  --json-out tests/artifacts/perf/qf-consultation-dormant-tanbear-20260507T1835Z/update-qf-dormant.json \
  --no-html
```

Result JSON: `update-qf-dormant.json`

Comparison reference: nearby clean focused baseline
`tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`.

| Metric | Baseline | QF dormant candidate |
| --- | ---: | ---: |
| Average ratio | 1.1677116353247705 | 1.205157732760332 |
| Geomean ratio | 1.1564512197233796 | 1.1831606791823142 |
| Median ratio | 1.1181199812385678 | 1.1142997500810805 |
| p90 / p99 ratio | 1.4506585635858391 | 1.6280431892164504 |
| Franken faster | 0 | 0 |
| Comparable | 2 | 2 |
| C SQLite faster | 4 | 4 |

Per-row ratios:

| Scenario | Baseline | QF dormant candidate |
| --- | ---: | ---: |
| 100 rows / update 10 rows | 1.4506585635858391 | 1.6280431892164504 |
| 100 rows / delete 5 rows | 1.338643120202334 | 1.4321583540539222 |
| 1000 rows / update 100 rows | 1.0240318257458014 | 1.1142997500810805 |
| 1000 rows / delete 50 rows | 1.0170495485422661 | 0.9834432429446066 |
| 10000 rows / update 1000 rows | 1.1181199812385678 | 1.0099381989558756 |
| 10000 rows / delete 500 rows | 1.0577667726338156 | 1.0630636613100561 |

## Decision

Rejected as a standalone performance optimization. It improves one delete row
and one large update row in this remeasure, but the focused section gate moves
the wrong way overall: average/geomean regress and the tail ratio worsens.

Do not retry explicit dormant QF consultation as a standalone optimization.
Reconsider only if the QF feature is removed or redesigned for semantic reasons
and a same-window update/delete plus full quick matrix shows neutral-or-better
results.

Patch-ready ledger note, if the ledger owner wants to add it:

```md
## 2026-05-07 - Explicitly dormant QF consultation

- Target: `UPDATE/DELETEThroughput`, especially direct-simple rowid UPDATE/DELETE
  calls that still consult `qf_maybe_short_circuit_for_rowid`.
- Candidate shape: add a disabled `QUOTIENT_FILTER_CONSULTATION_ENABLED` flag and
  return `Ok(false)` before borrowing the quotient-filter map, rewriting QF tests
  to assert normal B-tree lookup with zero short-circuits.
- Evidence: `tests/artifacts/perf/qf-consultation-dormant-tanbear-20260507T1835Z/`.
  Focused QF tests and `comprehensive-bench` build passed.
- Result: rejected. Against
  `tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`,
  focused update/delete geomean regressed `1.1564512197233796 ->
  1.1831606791823142`, average regressed `1.1677116353247705 ->
  1.205157732760332`, p90/p99 regressed `1.4506585635858391 ->
  1.6280431892164504`, and C-faster rows stayed `4`.
- Do not retry explicitly dormant QF consultation as a standalone optimization.
  Reconsider only as part of a semantic QF removal/redesign that also passes a
  same-window update/delete and full quick matrix.
```
