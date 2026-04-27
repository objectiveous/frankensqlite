# Direct INSERT stateless append-hint rejection

Agent: AzurePine
Date: 2026-04-27
Workload: `perf-update-delete 10000 <iters> both`
Decision: rejected and rolled back

## Profile basis

Baseline before the candidate was `70c3ad38 perf: publish btree sort record rejection`.

Profile build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-next3-profile \
  CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only \
  CARGO_PROFILE_RELEASE_PERF_STRIP=false \
  RUSTFLAGS='-C force-frame-pointers=yes' \
  cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Representative baseline timings:

| rows | iters | total | populate | update | delete |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10000 | 50 | 822ms | 411ms | 247ms | 122ms |
| 10000 | 50 | 821ms | 418ms | 236ms | 125ms |
| 10000 | 200 | 3262ms | 1629ms | 950ms | 524ms |

`perf report --no-children` showed direct INSERT and B-tree append work still visible:

| symbol | self |
| --- | ---: |
| `__memmove_avx_unaligned_erms` | 7.26% |
| `Connection::execute_prepared_direct_simple_insert` | 5.85% |
| `BtCursor<SharedTxnPageIo>::delete` | 5.77% |
| `sort_unstable_by<sort_cells_desc_by_ptr>` | 3.90% |
| `small_sort_general<sort_cells_desc_by_ptr>` | 3.19% |
| `_int_malloc` | 3.00% |
| `fsqlite_mvcc::begin_concurrent::concurrent_page_state` | 2.35% |
| `SharedTxnPageIo::write_page_internal` | 2.09% |
| `WalChecksumTransform::for_wal_frame` | 2.09% |

## Candidate

The candidate added a stateless direct-INSERT append-hint path:

- keep the existing `table_try_append_rightmost_leaf_hint_known_last_rowid_with_state` behavior for callers that retain page bytes;
- for explicit transactions, call a new stateless path that still stages the mutated leaf via `write_page_data`;
- return `TableAppendHint { page_data: None, parent_page: None, ... }` so the caller does not pay for a retained page-image clone it will immediately discard.

This did not change concurrent-writer defaults, did not add file-level writer serialization, and still staged through the existing MVCC page writer.

Focused behavior checks passed while the candidate was present:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-stateless-hint-test \
  cargo test -p fsqlite-btree test_table_try_append_rightmost_leaf_hint_known_last_rowid_with_state_refreshes_hint -- --nocapture

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-stateless-hint-core-test \
  cargo test -p fsqlite-core test_prepared_direct_simple_insert_implicit_rowid_retains_append_hint_in_explicit_txn -- --nocapture
```

Both passed.

## Isolated A/B on `70c3ad38`

To avoid peer-work contamination, the candidate was also built in a detached worktree at `70c3ad38` with only the candidate patch applied.

Baseline binary:

```text
/data/tmp/cargo-target-azurepine-20260427-next3-profile/release-perf/perf-update-delete
```

Candidate binary:

```text
/data/tmp/cargo-target-azurepine-20260427-stateless-hint-isolated/release-perf/perf-update-delete
```

| iters | metric | baseline avg | candidate avg | result |
| ---: | --- | ---: | ---: | ---: |
| 50 | total | 816.0ms | 824.3ms | +1.0% slower |
| 50 | populate | 410.3ms | 418.3ms | +1.9% slower |
| 100 | total | 1587.7ms | 1618.0ms | +1.9% slower |
| 100 | populate | 810.0ms | 821.0ms | +1.4% slower |
| 200 | total | 3199.7ms | 3250.0ms | +1.6% slower |
| 200 | populate | 1622.3ms | 1650.3ms | +1.7% slower |

Raw isolated runs:

```text
base batch=50 run=1 total=808ms populate=409ms update=236ms delete=124ms
cand batch=50 run=1 total=822ms populate=414ms update=243ms delete=125ms
base batch=50 run=2 total=819ms populate=414ms update=240ms delete=125ms
cand batch=50 run=2 total=830ms populate=423ms update=241ms delete=126ms
base batch=50 run=3 total=821ms populate=408ms update=249ms delete=125ms
cand batch=50 run=3 total=821ms populate=418ms update=236ms delete=125ms
base batch=100 run=1 total=1593ms populate=832ms update=474ms delete=218ms
cand batch=100 run=1 total=1608ms populate=823ms update=485ms delete=227ms
base batch=100 run=2 total=1591ms populate=801ms update=465ms delete=251ms
cand batch=100 run=2 total=1621ms populate=819ms update=485ms delete=243ms
base batch=100 run=3 total=1579ms populate=797ms update=461ms delete=248ms
cand batch=100 run=3 total=1625ms populate=821ms update=482ms delete=250ms
base batch=200 run=1 total=3229ms populate=1625ms update=951ms delete=510ms
cand batch=200 run=1 total=3235ms populate=1640ms update=961ms delete=486ms
base batch=200 run=2 total=3169ms populate=1628ms update=935ms delete=467ms
cand batch=200 run=2 total=3240ms populate=1638ms update=964ms delete=492ms
base batch=200 run=3 total=3201ms populate=1614ms update=931ms delete=512ms
cand batch=200 run=3 total=3275ms populate=1673ms update=968ms delete=487ms
```

## Current-HEAD A/B on `62c3ecc4`

During the isolation pass, peer work landed as `62c3ecc4 fix(btree): reject invalid non-empty cell content offsets`. A fresh current-HEAD baseline was built and compared with the candidate binary built from the same cell-offset code plus the stateless-hint patch.

Baseline binary:

```text
/data/tmp/cargo-target-azurepine-20260427-head-62c3ecc/release-perf/perf-update-delete
```

Candidate binary:

```text
/data/tmp/cargo-target-azurepine-20260427-stateless-hint/release-perf/perf-update-delete
```

| iters | metric | baseline avg | candidate avg | result |
| ---: | --- | ---: | ---: | ---: |
| 50 | total | 801.5ms | 811.0ms | +1.2% slower |
| 50 | populate | 410.0ms | 415.5ms | +1.3% slower |
| 100 | total | 1583.0ms | 1584.0ms | neutral |
| 100 | populate | 810.5ms | 831.5ms | +2.6% slower |
| 200 | total | 3202.0ms | 3231.5ms | +0.9% slower |
| 200 | populate | 1633.0ms | 1669.0ms | +2.2% slower |

Raw current-HEAD runs:

```text
base-current batch=50 run=1 total=797ms populate=416ms update=230ms delete=118ms
cand-current batch=50 run=1 total=801ms populate=408ms update=236ms delete=123ms
base-current batch=50 run=2 total=806ms populate=404ms update=243ms delete=125ms
cand-current batch=50 run=2 total=821ms populate=423ms update=234ms delete=126ms
base-current batch=100 run=1 total=1574ms populate=798ms update=459ms delete=250ms
cand-current batch=100 run=1 total=1591ms populate=835ms update=469ms delete=221ms
base-current batch=100 run=2 total=1592ms populate=823ms update=470ms delete=235ms
cand-current batch=100 run=2 total=1577ms populate=828ms update=469ms delete=217ms
base-current batch=200 run=1 total=3203ms populate=1631ms update=953ms delete=485ms
cand-current batch=200 run=1 total=3260ms populate=1704ms update=919ms delete=499ms
base-current batch=200 run=2 total=3201ms populate=1635ms update=926ms delete=509ms
cand-current batch=200 run=2 total=3203ms populate=1634ms update=929ms delete=506ms
```

## Conclusion

The lever was plausible from the profile, but timings rejected it. It removes an apparent clone, yet the direct-INSERT populate phase regressed on both isolated and current-HEAD comparisons. The source candidate was rolled back; only this rejection artifact remains.

Next better targets from the same profile:

- table-delete cell movement / sort work;
- `execute_prepared_direct_simple_insert` without changing the append-hint retained-state contract;
- allocator pressure in `write_page_internal` / MVCC staging.
