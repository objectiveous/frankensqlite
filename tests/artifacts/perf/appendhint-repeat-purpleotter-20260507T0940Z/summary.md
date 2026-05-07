# Retained append-hint candidate repeat

Date: 2026-05-07
Agent: PurpleOtter
Status: read-only repeat evidence for CrimsonGorge's reserved
`crates/fsqlite-core/src/connection.rs` candidate.

## Candidate Under Test

The shared checkout had an uncommitted `connection.rs` diff that preserves
`Connection::prepared_direct_insert_append_hint` when
`invalidate_cached_write_txn` finds no cached writer to invalidate.

Diff fingerprint:
`cc7df21f0579646edd5303c3cf8a5cff6e6642ef2d2dc19539a319fcc5ed17ed`

I did not edit, stage, or commit `connection.rs`; CrimsonGorge held the
exclusive reservation on that file.

## Command

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-appendhint-crimsongorge-release/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/appendhint-repeat-purpleotter-20260507T0940Z/candidate-full-repeat.json \
  --no-html
```

## Result

The full quick matrix completed successfully. This is important because the
previous right-edge byte-slice payload append candidate failed during repeat
full-matrix concurrent-writer rows.

Summary from `candidate-full-repeat.json`:

- Total scenarios: `93`
- Franken faster / comparable / C faster: `77 / 6 / 10`
- Average ratio: `0.4922625798006523`
- Geomean ratio: `0.281842116428279`
- Median ratio: `0.3206578171432536`
- P90 / P99 ratio: `1.0671974460298612 / 1.7635604325621839`
- Primary weighted score: `0.3679977906976089`
- Write-single geomean: `1.061066773304065`
- Write-bulk geomean: `0.8911259169069496`
- Concurrent-writers geomean: `0.715596838789969`

Transaction-strategy rows:

| Scenario | Ratio | FSQLite median ms | C SQLite median ms |
| --- | ---: | ---: | ---: |
| 100 rows / autocommit | `1.0967945907665329` | `0.130093` | `0.118612` |
| 100 rows / batched (100/txn) | `1.0671974460298612` | `0.109646` | `0.102742` |
| 100 rows / single txn | `0.9938238888948979` | `0.091882` | `0.092453` |
| 1000 rows / autocommit | `0.8788384758496712` | `0.727934` | `0.828291` |
| 1000 rows / batched (1000/txn) | `0.8463885084273797` | `0.29909` | `0.353372` |
| 1000 rows / single txn | `0.7587995299456182` | `0.298319` | `0.393146` |
| 10000 rows / autocommit | `0.8273522435245938` | `6.83398` | `8.260061` |
| 10000 rows / batched (1000/txn) | `1.3117447236427406` | `4.278544` | `3.26172` |
| 10000 rows / single txn | `0.7482354949804996` | `2.401598` | `3.209682` |

## Interpretation

This repeat supports the targeted write-single/autocommit improvement and shows
the candidate is not failing the concurrent-writer repeat path. It does not, by
itself, reproduce CrimsonGorge's first candidate full-matrix primary-score win:
the repeat primary score `0.3679977906976089` is slightly worse than the
earlier same-window baseline primary score `0.3673037319768524` in
`tests/artifacts/perf/appendhint-crimsongorge-20260507T0925Z/baseline-full.json`.

I would treat this as supportive but not sufficient standalone keep evidence:
the source holder should either land with the earlier same-window A/B plus this
repeat-stability evidence, or run a fresh baseline/candidate pair before final
commit if strict primary-score repeat is required.
