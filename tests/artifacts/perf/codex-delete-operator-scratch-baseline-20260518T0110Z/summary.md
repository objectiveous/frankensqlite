# DELETE Operator Scratch Baseline - 2026-05-18

## Command

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-operator-target-20260518T0110Z \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update-delete \
  --json-out tests/artifacts/perf/codex-delete-operator-scratch-baseline-20260518T0110Z/update-delete.json \
  --no-html
```

The command ran in clean scratch checkout
`/data/tmp/frankensqlite-delete-operator-scratch-2dad5c28-20260518T0110Z`
created from `main @ 2dad5c28`. RCH fell back local because no worker was
admissible (`critical_pressure=6`).

## Focused Result

| Scenario | C SQLite | FrankenSQLite | F/C | CV C | CV F |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.005711 ms | 0.009188 ms | 1.6088x | 98.1% | 61.4% |
| 100 rows / delete 5 rows | 0.003086 ms | 0.013646 ms | 4.4219x | 74.9% | 40.0% |
| 1000 rows / update 100 rows | 0.040386 ms | 0.041818 ms | 1.0355x | 8.8% | 47.1% |
| 1000 rows / delete 50 rows | 0.038211 ms | 0.053561 ms | 1.4017x | 67.3% | 35.1% |
| 10000 rows / update 1000 rows | 0.520665 ms | 0.333114 ms | 0.6398x | 20.5% | 23.6% |
| 10000 rows / delete 500 rows | 0.224500 ms | 0.315661 ms | 1.4061x | 16.7% | 20.5% |

Summary: 6 scenarios, FrankenSQLite faster / comparable / C-SQLite-faster at
`1 / 1 / 4`, average F/C `1.7522921813538774`, write-single geomean
`1.4498564588938154`.

The profiled 10K DELETE row still shows the same distributed surface:
`delete_active_probe_ns=166612`, `delete_leaf_flush_ns=67526`,
`delete_leaf_search=560/47894`, `delete_leaf_materialize=64/52305`,
`delete_seek_ns=43378`, and `commit_roundtrip_ns=24105`.
