# Direct UPDATE Lazy Scratch Borrow Candidate

Date: 2026-05-08
Agent: BoldLion
Baseline source commit: `536dc300512ff37adb47e36b6ed9150dea3f8e1d`

## Candidate

The temporary source diff in `crates/fsqlite-core/src/connection.rs` delayed
borrowing `prepared_direct_update_row_scratch` until after the fixed-width REAL
direct UPDATE fast path declined. The benchmark UPDATE statement takes that
fixed-width path, so the candidate removed one `RefCell` borrow from each hot
UPDATE row without changing DELETE, page I/O selection, cursor retention, or
concurrent-writer defaults.

The source diff was restored after the repeat focused gate rejected it.

## Correctness

Commands:

```text
cargo fmt -p fsqlite-core --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-dml-current-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_all_non_ipk_columns_skips_old_payload_decode -- --nocapture --test-threads=1
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-dml-current-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture --test-threads=1
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-dml-current-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture --test-threads=1
```

All three focused tests passed when run serially. A broad
`direct_simple_update` filter was not used as correctness evidence because
parallel tests share global hot-path counters and one profile assertion saw
cross-test counter noise.

## Focused UPDATE/DELETE Gate

Artifacts:

- `head-dml-profile.json`: current clean focused baseline.
- `candidate-lazy-update-scratch.json`: first dirty candidate run.
- `candidate-lazy-update-scratch-repeat.json`: immediate dirty candidate repeat.

| Metric | Baseline | Candidate | Candidate repeat |
| --- | ---: | ---: | ---: |
| Average ratio | `1.1203914638` | `1.0344792667` | `1.1957309163` |
| Geomean ratio | `1.1122084670` | `1.0225809144` | `1.1693177146` |
| P90/P99 ratio | `1.3766795485` | `1.3514169105` | `1.6946494052` |
| Faster / comparable / slower | `0 / 3 / 3` | `1 / 4 / 1` | `0 / 3 / 3` |

Key rows:

| Row | Baseline ratio | Candidate ratio | Repeat ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | `1.1659530910` | `1.0371831595` | `1.3924088986` |
| 100 rows / delete 5 rows | `1.3766795485` | `1.3514169105` | `1.6946494052` |
| 1000 rows / update 100 rows | `1.0095502770` | `1.0245922093` | `1.1113679431` |
| 10000 rows / update 1000 rows | `1.0396020883` | `1.0199341318` | `1.0353690709` |

## Decision

Rejected and restored. The first run looked promising, but the immediate repeat
failed the focused DML gate: average/geomean regressed versus the current clean
baseline, p90/p99 worsened, and the 100-row update/delete tails both moved the
wrong way. The avoided `RefCell` borrow is below the noise floor for this
section.

Do not retry lazy borrowing of the direct UPDATE row-value scratch as a
standalone optimization. Reconsider only if it falls out naturally inside a
broader DML run operator that removes larger per-row admission or mutation work
and wins repeated focused UPDATE/DELETE gates.
