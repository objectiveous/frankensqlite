# B-tree guard transaction profile

## Context

- Date: 2026-05-07 10:57 UTC
- Commit: `f05e02d5f60add217c12cb39719c8f63d5fb1695`
- Worktree: dirty
- Dirty files captured in `dirty.diff`:
  - `crates/fsqlite-btree/src/cursor.rs`
  - `docs/progress/perf-negative-results.md`
- Binary: `/data/tmp/frankensqlite-crimsongorge-btreeguard-target/release-perf/comprehensive-bench`
- Profile mode: `FSQLITE_BENCH_PROFILE_INSERT=1`
- Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-crimsongorge-btreeguard-target/release-perf/comprehensive-bench \
  --quick --filter transaction \
  --json-out tests/artifacts/perf/btreeguard-transaction-crimsongorge-20260507T105710Z/report-transaction.json \
  --no-html
```

## Result

- Scenarios: 9
- FrankenSQLite faster: 6
- Comparable: 0
- C SQLite faster: 3
- Average ratio: `0.9773664779734071`
- Geomean ratio: `0.9620317870341055`
- Median ratio: `0.8643561826182927`
- p90 ratio: `1.2935537008764353`
- p99 ratio: `1.2935537008764353`
- Observed primary score: `0.8980467554913775`

## Remaining C-faster rows

| Scenario | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| `10000 rows / batched (1000/txn)` | `3.318214 ms` | `4.292288 ms` | `1.2935537008764353` |
| `100 rows / batched (100/txn)` | `0.076022 ms` | `0.093065 ms` | `1.2241851043119096` |
| `100 rows / single txn` | `0.077986 ms` | `0.090088 ms` | `1.1551816992793578` |

## Hot counters for `fs_insert_txn_batched_small_3col_10000`

- `insert_us=8942.7`
- `row_build_ns=1737799`
- `cursor_setup_ns=410185`
- `btree_insert_ns=1612862`
- `btree_leaf_payload_appends=8934`
- `btree_leaf_full_cell_appends=9`
- `btree_leaf_payload_mutate_ns=314756`
- `btree_quick_balance_attempts=57`
- `btree_quick_balance_hits=57`
- `btree_quick_balance_ns=104494`
- `btree_conservative_reloads=57`
- `commit_roundtrip_ns=128832`
- `schema_validation_ns=407814`
- `change_tracking_ns=242503`

## Interpretation

This is a dirty-worktree validation run of PurpleOtter's B-tree cursor guard, not a clean landed A/B. It is useful because it confirms the guard still passes the focused B-tree correctness test and the transaction profile remains dominated by the same non-empty batched right-edge append row.

Compared with the earlier clean insert profile artifact, the worst transaction row is less severe here (`4.292288 ms`, `1.2936x` over C) than the clean insert profile's `10000 rows / batched (1000/txn)` row (`4.666265 ms`, `1.4463x` over C). Because the run is dirty and same-window noise is uncontrolled, treat this as supportive evidence for landing the guard as a correctness prerequisite, not as proof that the guard alone closes the performance gap.

The next high-EV optimization remains a true non-empty page builder or fused record-body plus page-layout builder that preserves the payload-append kernel. The current negative ledger fences the append-hint-started route if flush falls back to row-at-a-time full-cell replay.
