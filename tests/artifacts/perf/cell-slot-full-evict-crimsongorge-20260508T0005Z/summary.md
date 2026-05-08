# CellSlotCache full-entry pre-evict rejection

## Target

`comprehensive-bench --quick --filter update`, after the current mixed
write-profile showed
`RawVec<CellSlotCacheEntry>::grow_one` at 0.65% self time and the remaining
full quick matrix still had slow small UPDATE/DELETE rows.

## Candidate

In `crates/fsqlite-btree/src/cursor.rs`, change
`CellSlotCache::insert_slow` so a new entry insertion on a full 64-entry cache
pops the tail before `Vec::insert(0, entry)`. The intended equivalence was:
`insert new MRU then truncate tail` == `pop tail then insert new MRU`, while
avoiding a transient growth from 64 to 128 large `CellSlotCacheEntry` values.

## Proof gates

- `cargo fmt -p fsqlite-btree --check`: passed before the candidate benchmark.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cursor-candidate-baseline-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-btree cell_slot_cache_evicts_tail_before_full_new_entry_insert -- --nocapture`: passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cell-slot-full-evict-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`: passed.

The source was manually restored after the focused benchmark rejected the
candidate.

## Benchmark evidence

Baseline:

- Average ratio: `0.9809793061876931`
- Geomean ratio: `0.9659518881677094`
- Median ratio: `0.9250150432399906`
- P90/P99 ratio: `1.385192596298149`

Candidate:

- Average ratio: `1.172453260226592`
- Geomean ratio: `1.1619765793212873`
- Median ratio: `1.122884642109463`
- P90/P99 ratio: `1.4204116540281657`

Rows:

| Scenario | Baseline F ms | Candidate F ms | Baseline ratio | Candidate ratio |
|---|---:|---:|---:|---:|
| 100 rows / update 10 rows | `0.123381` | `0.124563` | `0.824629060286058` | `1.4204116540281657` |
| 100 rows / delete 5 rows | `0.116298` | `0.118422` | `1.385192596298149` | `1.3550513198997631` |
| 1000 rows / update 100 rows | `0.407533` | `0.427521` | `0.9746935046422746` | `1.0310978300978952` |
| 1000 rows / delete 50 rows | `0.372017` | `0.398947` | `0.9250150432399906` | `0.9933889771465282` |
| 10000 rows / update 1000 rows | `3.450183` | `4.230475` | `0.8974752628617642` | `1.1118851380777375` |
| 10000 rows / delete 500 rows | `3.184295` | `4.007728` | `0.8788703697979227` | `1.122884642109463` |

## Verdict

Rejected. The candidate removed one plausible capacity-growth cliff but made
the focused UPDATE/DELETE section worse, especially the 10K rows. Do not retry
full-cache pre-eviction as a standalone `CellSlotCache` micro-optimization.
Reconsider only if a future profile proves the 64-to-128 growth itself dominates
and the replacement changes the cache structure more fundamentally, with a full
matrix gate.
