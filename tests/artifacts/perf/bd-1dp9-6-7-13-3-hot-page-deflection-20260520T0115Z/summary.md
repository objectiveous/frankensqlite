# bd-1dp9.6.7.13.3 Hot-Page Deflection Evidence

Date: 2026-05-20

## Code Path

- Policy: `btree.hot_page_deflection.v1`
- Trigger: page heat >= 64 and writer overlap >= 4
- Budget: 2 split-time deflection credits per armed page
- Reversibility: `FSQLITE_CONFLICT_TOPOLOGY_POLICY=baseline` forces baseline placement and reports `operator_override_baseline`
- Layout discipline: no background page migration or physical recluster is performed; the mitigation only biases the next table-leaf split target and then falls back to the existing topology-aware split target when credits are exhausted

## Synthetic Bad-Hotspot Proof

Commands:

```text
rch exec -- cargo test -p fsqlite-btree hot_page_deflection -- --nocapture
rch exec -- cargo test -p fsqlite-btree test_pathological_conflict_heat_deflects_leaf_table_split_once_bounded -- --nocapture
```

Artifacts:

- `hot_page_deflection_tests.txt`: 2/2 tests passed
- `balance_policy_hotspot_test.txt`: 1/1 test passed

The synthetic workload feeds 64 repeated heat observations with overlap 4 into one page. The first two split decisions consume bounded deflection credits and target a 9000 basis-point right-edge split; the third decision reports `budget_exhausted` and falls back to the existing 8000 basis-point topology-aware target.

## Concurrent Writer Benchmark

Command shape:

```text
FSQLITE_CONFLICT_TOPOLOGY_POLICY={baseline,enforced} rch exec -- cargo run --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- --rows-per-thread=200 --threads=8 --iters=5 ...
```

Final 8-thread shared-table rows:

| Policy | fsqlite p50 ms | fsqlite p95 ms | fsqlite p99 ms | fsqlite p50 wps | vs SQLite | failed rows |
|---|---:|---:|---:|---:|---:|---:|
| baseline | 11.324 | 13.132 | 13.369 | 141296 | 7.120x | 0 |
| enforced | 11.558 | 14.695 | 15.107 | 138435 | 6.969x | 0 |

This broad mt-mvcc row is a no-claim result for `.13.3`: concurrent-writer mode remains strong versus SQLite, but the new escape hatch is justified by the deliberately pathological synthetic proof rather than a p50/p95/p99 win on this small shared-table benchmark.
