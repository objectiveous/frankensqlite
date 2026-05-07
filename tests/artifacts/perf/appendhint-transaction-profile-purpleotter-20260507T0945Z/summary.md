# Transaction profile after retained append-hint candidate

Date: 2026-05-07
Agent: PurpleOtter
Status: read-only profile of CrimsonGorge's reserved retained append-hint
candidate in `crates/fsqlite-core/src/connection.rs`.

## Command

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-appendhint-crimsongorge-release/release-perf/comprehensive-bench \
  --quick \
  --filter transaction \
  --json-out tests/artifacts/perf/appendhint-transaction-profile-purpleotter-20260507T0945Z/report-transaction.json \
  --no-html
```

Stdout and stderr were captured in this artifact directory.

## Section Result

- Total scenarios: `9`
- Franken faster / comparable / C faster: `6 / 0 / 3`
- Primary weighted score: `0.9052686026553983`
- Geomean ratio: `0.9597768430670817`
- Write-single geomean: `0.8740610598153417`
- Write-bulk geomean: `1.0057372016816681`

| Scenario | Ratio | FSQLite median ms | C SQLite median ms |
| --- | ---: | ---: | ---: |
| 100 rows / autocommit | `0.8694786226431797` | `0.11787` | `0.135564` |
| 100 rows / batched (100/txn) | `1.111172615078816` | `0.088326` | `0.079489` |
| 100 rows / single txn | `1.163124413114494` | `0.087945` | `0.075611` |
| 1000 rows / autocommit | `0.8776839501768207` | `0.728174` | `0.829654` |
| 1000 rows / batched (1000/txn) | `0.8565789841500573` | `0.306424` | `0.35773` |
| 1000 rows / single txn | `0.8622115547296851` | `0.300673` | `0.348723` |
| 10000 rows / autocommit | `0.8750406970785503` | `6.958363` | `7.952045` |
| 10000 rows / batched (1000/txn) | `1.3290062724722278` | `4.289494` | `3.227595` |
| 10000 rows / single txn | `0.8158139048588402` | `2.590542` | `3.175408` |

## Hot-Path Counters

The retained append-hint candidate moved autocommit onto the intended payload
append path:

- `10000 rows / autocommit`: `btree_cell_assembly_calls=164`,
  `btree_leaf_payload_appends=9898`, `btree_leaf_full_cell_appends=39`.
- `1000 rows / autocommit`: `btree_cell_assembly_calls=17`,
  `btree_leaf_payload_appends=989`, `btree_leaf_full_cell_appends=5`.
- `100 rows / autocommit`: `btree_cell_assembly_calls=2`,
  `btree_leaf_payload_appends=98`, `btree_leaf_full_cell_appends=2`.

The remaining loser is `10000 rows / batched (1000/txn)`:

- `insert_us=8468.0`
- `row_build_ns=1598670`
- `cursor_setup_ns=410210`
- `btree_insert_ns=1487546`
- `btree_cell_assembly_calls=123`
- `btree_leaf_payload_appends=8934`
- `btree_quick_balance_hits=57`

For comparison, `10000 rows / single txn` on the same run:

- `insert_us=5765.3`
- `row_build_ns=1433647`
- `cursor_setup_ns=1062`
- `btree_insert_ns=319574`
- `btree_cell_assembly_calls=0`
- `btree_leaf_payload_appends=0`

## Interpretation

After the append-hint candidate, autocommit is no longer the main transaction
strategy gap. The remaining high-EV row is `10000 rows / batched (1000/txn)`.
Its loss is not explicit BEGIN/COMMIT time (`begin_us=42.7`, `commit_us=174.2`);
it is the direct INSERT body after the first empty-root batch stops using the
single-transaction bulk/page-run shape.

This points to a future non-empty-root bulk append/run builder. The negative
ledger already rejects connection-only page-run threshold changes,
retained-autocommit page-run widening, and standalone direct-record layout
reuse, so the next candidate should not be another threshold toggle or isolated
row-build reshuffle. It needs to batch non-empty right-edge appends, or fuse row
layout into such a page builder, with a correctness proof before matrix
measurement.
