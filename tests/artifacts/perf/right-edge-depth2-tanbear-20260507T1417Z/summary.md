# Depth-2 Right-Edge Page-Builder Read-Only A/B

- Agent: TanBear
- Date: 2026-05-07
- Baseline commit: `60e1434f5d001180ce4258e1dfb34be55c06036f`
- Candidate source state: dirty shared worktree at the same commit.
- Candidate owner: CrimsonGorge. `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs` were exclusively reserved by
  CrimsonGorge during this measurement. TanBear did not edit, stage, or revert
  those files.

## Candidate Shape

The dirty candidate adds a depth-2 right-edge bulk append/page-builder path for
prepared direct INSERT page-runs. This is the retry condition allowed by the
negative ledger for the `10000 rows / batched (1000/txn)` transaction gap: it
builds and splices whole right-edge leaf pages and parent divider cells, instead
of replaying buffered rows through the existing writer-flush append loop.

Legacy SQLite uses `BTREE_APPEND` plus `balance_quick()` for append-biased table
inserts. This candidate is more batch-oriented: it applies a B-epsilon style
message-run idea by materializing an entire monotone right-edge run into pages
at the transaction boundary.

## Commands

Clean full quick baseline:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-tanbear-clean-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick \
  --json-out /data/tmp/frankensqlite-tanbear-clean-fullquick.json \
  --no-html
```

Dirty full quick candidate:

```bash
/data/tmp/frankensqlite-tanbear-target/release-perf/comprehensive-bench \
  --quick \
  --json-out /data/tmp/frankensqlite-tanbear-dirty-fullquick.json \
  --no-html
```

Dirty transaction repeat:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-tanbear-target/release-perf/comprehensive-bench \
  --quick \
  --filter transaction \
  --json-out /data/tmp/frankensqlite-tanbear-transaction-dirty-repeat.json \
  --no-html
```

Dirty update/delete profile:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-tanbear-target/release-perf/comprehensive-bench \
  --quick \
  --filter update \
  --json-out /data/tmp/frankensqlite-tanbear-dirty-update-profile.json \
  --no-html
```

Correctness smoke tests:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-tanbear-test-target \
  cargo test -p fsqlite-btree \
  test_table_bulk_append_depth2_right_edge_sorted_records_extends_tree -- --nocapture

CARGO_TARGET_DIR=/data/tmp/frankensqlite-tanbear-test-target \
  cargo test -p fsqlite-core \
  test_prepared_direct_insert_page_run_buffers_nonempty_right_edge_batch -- --nocapture
```

Both tests passed.

## Full Quick Same-Window A/B

| Metric | Clean | Dirty |
|---|---:|---:|
| Primary weighted score | 0.370335 | 0.368076 |
| Geomean ratio | 0.286622 | 0.280117 |
| F faster / comparable / C faster | 74 / 3 / 16 | 75 / 4 / 14 |

Lower score/ratio is better. In the same-window full quick run, the dirty
candidate modestly improved the overall primary score and geomean.

## Target Row

| Row | Clean C ms | Clean F ms | Clean F/C | Dirty C ms | Dirty F ms | Dirty F/C |
|---|---:|---:|---:|---:|---:|---:|
| `10000 rows / batched (1000/txn)` | 3.303899 | 4.475514 | 1.354616 | 3.211907 | 2.502868 | 0.779247 |

This closes the measured transaction-strategy target row in the same-window
quick matrix.

## Other Large INSERT Movements

Wins:

| Row | Clean F ms | Dirty F ms |
|---|---:|---:|
| `medium_6col` 10000 single-txn | 8.046263 | 4.622310 |
| `small_3col` 10000 single-txn | 3.674323 | 2.999539 |

Regressions worth repeating before final keep:

| Row | Clean F ms | Dirty F ms |
|---|---:|---:|
| record-size `large_10col` | 11.490967 | 11.948885 |
| record-size `small_3col` | 2.454098 | 2.681544 |
| `medium_6col` 1000 single-txn | 0.542947 | 0.758571 |

## UPDATE/DELETE Profile Read

The remaining `UPDATE/DELETEThroughput` gaps are not cleanly explained by the
direct mutation loops alone. In the dirty update profile:

- `10000 rows / update 1000 rows`: setup `2603.9 us`, mutate `1372.1 us`,
  commit `242.3 us`.
- `10000 rows / delete 500 rows`: setup `2574.1 us`, mutate `922.3 us`,
  commit `203.6 us`.
- `100 rows / update 10 rows`: setup `62.5 us`, mutate `13.2 us`,
  commit `6.4 us`.
- `100 rows / delete 5 rows`: setup `59.1 us`, mutate `9.1 us`,
  commit `6.1 us`.

The negative ledger already rejects most standalone direct UPDATE/DELETE
micro-optimizations. A future update/delete pass should profile setup and commit
separately before retrying direct rowid mutation ideas.

## Artifacts

- `clean-fullquick.json`
- `dirty-fullquick.json`
- `dirty-transaction-repeat.json`
- `dirty-update-profile.json`

## Recommendation

This is a plausible keeper candidate because it improves the intended
transaction row and the same-window full quick score. Before landing, repeat or
inspect the regressed INSERT rows above and publish source-owned artifacts from
the final candidate state.
