# Dense Retained DELETE Leaf Search Candidate - 2026-05-18

## Candidate

Scratch-only source patch in
`/data/tmp/frankensqlite-delete-operator-scratch-2dad5c28-20260518T0110Z`:
`crates/fsqlite-btree/src/cursor.rs` changed
`TableLeafDeleteRun::search_table_leaf` to mirror the normal cursor's dense
integer-key table-leaf proof. It checks cancellation first, then tests the
first and last rowids, computes a direct dense slot when
`first + cell_count - 1 == last`, verifies that slot, and falls back to the
existing binary search otherwise.

This is intentionally a leaf-search candidate, not the broader transaction-local
DML mutation operator. It was measured because the ordinary cursor already has
the dense proof while retained DELETE runs were still using plain binary search.

## Correctness

Initial test run exposed a cancellation-order bug: the dense precheck read leaf
rowids before honoring a cancelled `Cx`, and
`test_table_leaf_delete_run_honors_cancelled_context_before_search` failed.
The scratch patch was fixed by calling `observe_cursor_cancellation(cx)?` at the
start of `search_table_leaf`.

After the fix:

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-operator-target-20260518T0110Z \
  cargo test -p fsqlite-btree --lib table_leaf_delete_run
```

passed all 5 retained DELETE leaf-run tests.

## Benchmark

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-operator-target-20260518T0110Z \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update-delete \
  --json-out tests/artifacts/perf/codex-delete-dense-leaf-search-candidate-20260518T0112Z/update-delete.json \
  --no-html
```

RCH fell back local (`critical_pressure=6`).

| Scenario | Baseline F | Candidate F | Baseline F/C | Candidate F/C |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.009188 ms | 0.009658 ms | 1.6088x | 1.9205x |
| 100 rows / delete 5 rows | 0.013646 ms | 0.016611 ms | 4.4219x | 5.7777x |
| 1000 rows / update 100 rows | 0.041818 ms | 0.032020 ms | 1.0355x | 0.6596x |
| 1000 rows / delete 50 rows | 0.053561 ms | 0.048431 ms | 1.4017x | 1.9398x |
| 10000 rows / update 1000 rows | 0.333114 ms | 0.316323 ms | 0.6398x | 0.8411x |
| 10000 rows / delete 500 rows | 0.315661 ms | 0.257221 ms | 1.4061x | 1.2402x |

Profile counters moved in the expected local direction for 10K DELETE:
`delete_leaf_search` dropped from `560/47894` to `560/17797`,
`delete_active_probe_ns` dropped from `166612` to `120213`, and
`delete_leaf_flush_ns` dropped from `67526` to `58760`.

## Decision

Rejected and not promoted. The candidate improved the stable 10K DELETE F-side
median and reduced retained-leaf search counters, but it failed the focused keep
gate: 100-row DELETE regressed from `0.013646 ms` to `0.016611 ms`, the focused
average ratio worsened from `1.7522921813538774` to `2.063169978211233`, and
write-single geomean worsened from `1.4498564588938154` to
`1.5671143028808132`.

Do not retry dense retained-leaf search as a standalone patch. It confirms the
ledger's prior conclusion: reducing search can help a local counter and the 10K
F-side median, but the remaining DELETE gap is still distributed across retained
run publication, flush/materialization, transaction envelope, and cross-leaf
cursor lifetime.
