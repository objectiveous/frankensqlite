# DML Operator Boundary Refresh - 2026-05-17

## Command

```bash
mkdir -p tests/artifacts/perf/codex-dml-operator-boundary-head-20260517Tnext && \
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-boundary-head-20260517 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update-delete \
  --json-out tests/artifacts/perf/codex-dml-operator-boundary-head-20260517Tnext/update-delete.json \
  --no-html 2>&1 | tee tests/artifacts/perf/codex-dml-operator-boundary-head-20260517Tnext/run.log
```

- Source: `main @ f9bfbb1c688fc1fc1d3b06d8636cde767e3afcfe`
- Build: `release-perf`
- RCH: local fallback (`no admissible workers: critical_pressure=6`)
- Note: benchmark header reports `Git dirty: yes` because this checkout has
  ignored/untracked local artifacts, not because of a tracked source diff.

## Focused Result

| Scenario | C SQLite | FrankenSQLite | F/C |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | `3.978 us` | `5.931 us` | `1.4910x` |
| 100 rows / delete 5 rows | `2.144 us` | `17.152 us` | `8.0000x` |
| 1000 rows / update 100 rows | `70.081 us` | `56.145 us` | `0.8011x` |
| 1000 rows / delete 50 rows | `15.669 us` | `58.119 us` | `3.7092x` |
| 10000 rows / update 1000 rows | `465.863 us` | `268.232 us` | `0.5758x` |
| 10000 rows / delete 500 rows | `209.482 us` | `358.170 us` | `1.7098x` |

Summary: `6` scenarios, FrankenSQLite faster / comparable / C SQLite faster:
`2 / 0 / 4`, average F/C `2.71x`.

The 100-row DELETE row is too noisy in this run (`F` CV `98.4%`) to use as a
standalone source-selection signal. The stable conclusion matches the prior
post-active-probe profile: UPDATE is already green at medium/large sizes, while
prepared direct DELETE remains the durable red path.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Decision |
| --- | ---: | ---: | ---: | ---: | --- |
| Standalone retained DELETE search/probe/materialization tweak | 2 | 1 | 2 | `1.0` | Reject; fenced by repeated focused/full-quick failures. |
| Standalone synced-root or MemDatabase invalidation tweak | 1 | 1 | 1 | `1.0` | Reject; current profile shows small distributed cost and prior lifecycle rejection. |
| Standalone fixed-width UPDATE patch-run tweak | 1 | 1 | 2 | `0.5` | Reject; 100-row UPDATE is fixed-cost/noisy and larger UPDATE is already green. |
| Transaction-local rowid/key DML mutation operator | 5 | 3 | 4 | `3.75` | Only viable DELETE source direction, but must be operator-scoped. |

## Source Boundary

The current retained DELETE path already:

- proves affected-row count row-by-row through the B-tree cursor,
- buffers same-leaf physical mutations,
- groups monotone cross-leaf runs,
- flushes before reads/savepoints/VDBE/commit,
- restores pending buffers on flush error.

A useful next implementation cannot be another leaf-run micro-patch. It needs a
transaction-local DML mutation operator that removes per-row retained-run
ceremony while preserving exact affected counts, duplicate/missing rowid
semantics, read-your-writes, rollback/savepoint ownership, schema/cache
invalidation, and MVCC publication. The keep gate remains: focused
UPDATE/DELETE medians improve in the same run window and the full quick primary
score is neutral or better.
