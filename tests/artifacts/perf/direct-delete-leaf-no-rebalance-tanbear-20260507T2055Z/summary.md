# Direct DELETE no-rebalance leaf primitive rejection

- Agent: TanBear
- Date: 2026-05-07T20:55Z
- Source baseline: `1ae5ebdb docs(perf): record rejected shared txn context probe`
- Candidate state: source reverted after focused DML gate rejected it.

## Candidate

Add a narrow `BtCursor` table-leaf DELETE primitive for the current row when:

- the cursor is on a table leaf,
- the target cell is not the leaf maximum,
- the leaf has more than one cell, so no rebalance is needed.

Direct-simple DELETE tried that primitive before falling back to generic
`cursor.delete()`. The idea was to avoid the generic separator/anchor ceremony
while preserving the accepted eager-defrag table leaf layout. This deliberately
did not retry the rejected single-freeblock DELETE shortcut or the rejected
top-stack clone-only cleanup.

## Proof Before Measurement

Passed:

```text
cargo fmt -p fsqlite-btree -p fsqlite-core --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fast-delete-test-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-btree test_table_delete_current_leaf_without_rebalance_deletes_nonmax_only -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fast-delete-test-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_update_delete -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fast-delete-perf-target CARGO_BUILD_JOBS=10 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete
```

## Focused Matrix

Commands:

```text
FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-current-write-profile-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/direct-delete-leaf-no-rebalance-tanbear-20260507T2055Z/baseline-update.json --no-html
FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-fast-delete-perf-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/direct-delete-leaf-no-rebalance-tanbear-20260507T2055Z/candidate-update.json --no-html
```

Summary:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Average ratio | `1.1771902588843643` | `1.2530378457971052` |
| Geomean ratio | `1.1533073165550498` | `1.225724573854313` |
| p90/p99 ratio | `1.536265171211715` | `1.5504331538281433` |
| Franken faster / comparable / C faster | `1 / 2 / 3` | `0 / 3 / 3` |

Rows:

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| `100 rows / update 10 rows` | `1.536265171211715` | `1.5504331538281433` | `0.131767` | `0.132438` |
| `100 rows / delete 5 rows` | `1.5033001213837751` | `1.4450483465600277` | `0.118893` | `0.116718` |
| `1000 rows / update 100 rows` | `0.9710021301525611` | `1.0355359720505295` | `0.405239` | `0.409628` |
| `1000 rows / delete 50 rows` | `0.9469115638972931` | `1.5362883066875153` | `0.353662` | `0.577712` |
| `10000 rows / update 1000 rows` | `1.067881758417603` | `0.9842781777567272` | `3.918669` | `3.603468` |
| `10000 rows / delete 500 rows` | `1.0377808082432374` | `0.9666431178996882` | `3.478524` | `3.301673` |

## Decision

Rejected and reverted. The target small DELETE row improved slightly
(`0.118893 ms -> 0.116718 ms`) and large rows improved, but the section gate
lost overall and `1000 rows / delete 50 rows` regressed badly
(`0.353662 ms -> 0.577712 ms`). This does not clear the keep gate.

## Patch-Ready Ledger Entry

The negative-results ledger was exclusively reserved by CrimsonGorge until
`2026-05-07T22:43:38Z`, so this entry was not applied directly:

```markdown
## 2026-05-07 - Direct DELETE no-rebalance leaf primitive

- Target: `UPDATE/DELETEThroughput`, especially direct-simple DELETE rows where
  generic `BtCursor::delete` pays separator/anchor ceremony even when the
  current leaf will remain non-empty and the deleted cell is not the leaf max.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`; source was manually restored after
  the focused DML gate rejected the change.
- Candidate shape: add a narrow table-leaf DELETE primitive that accepts only
  non-max table leaf cells on leaves with more than one cell, calls the existing
  eager-defrag `remove_table_cell_from_leaf_deferred`, and falls back to generic
  `delete()` for all structural/separator/rebalance cases.
- Evidence artifacts:
  `tests/artifacts/perf/direct-delete-leaf-no-rebalance-tanbear-20260507T2055Z/summary.md`,
  `baseline-update.json`, `candidate-update.json`, and `stdout/`.
- Result: rejected and reverted. Focused DML average/geomean worsened
  `1.1771902588843643 / 1.1533073165550498` to
  `1.2530378457971052 / 1.225724573854313`. The 100-row DELETE row improved
  slightly (`0.118893 ms -> 0.116718 ms`) and 10K rows improved, but
  `1000 rows / delete 50 rows` regressed sharply
  `0.353662 ms -> 0.577712 ms`, so the section keep gate failed.
- Do not retry non-max/no-rebalance table-leaf DELETE bypass as a standalone
  direct DELETE optimization. Reconsider only as part of a real same-leaf batch
  mutation primitive that writes each leaf once and proves an UPDATE/DELETE
  section geomean win.
```
