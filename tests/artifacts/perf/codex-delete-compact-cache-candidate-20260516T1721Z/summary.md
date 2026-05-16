# Direct DELETE Compact-Shape Cache Candidate

- Date: 2026-05-16
- Source: local tree based on `2de7fad121f2759befae76d8504ebb80a29f053f`
- Change under test: cache the retained table-leaf compact-cell-area predicate
  when a direct DELETE leaf run is captured, instead of recomputing the same
  predicate for every row accepted into that retained page image.
- Command:
  `rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-delete-compact-cache-candidate-20260516T1721Z/update-delete.json --no-html`
- Note: the benchmark reported a JSON output path, but local RCH retrieval left
  only an ignored empty `run.log`. This summary preserves the terminal output
  rows and counters.

## Result

Compared with the retained direct-delete cursor-reuse run in
`tests/artifacts/perf/codex-delete-flush-cursor-reuse-candidate-20260516T1604Z/summary.md`,
FrankenSQLite DELETE absolute time improved at the 1k and 10k row counts. The
100-row DELETE row was slightly slower, and UPDATE rows were noisy despite this
change not touching the UPDATE path.

| Scenario | C SQLite | FrankenSQLite | Ratio | Status |
|---|---:|---:|---:|---|
| `100 rows / update 10 rows` | `5.1 us` | `6.6 us` | `1.29x` | slower |
| `100 rows / delete 5 rows` | `2.9 us` | `7.1 us` | `2.46x` | slower |
| `1000 rows / update 100 rows` | `38.3 us` | `30.0 us` | `1.28x` | faster |
| `1000 rows / delete 50 rows` | `16.6 us` | `24.1 us` | `1.46x` | slower |
| `10000 rows / update 1000 rows` | `311.3 us` | `293.9 us` | `1.06x` | faster |
| `10000 rows / delete 500 rows` | `163.8 us` | `342.2 us` | `2.09x` | slower |

Summary statistics from the run: `6` scenarios, `2` FrankenSQLite-faster,
`0` comparable, `4` C-SQLite-faster, average time ratio `1.50x`.

## Key Counters

The DELETE path stayed on the direct prepared path:

- `fs_delete_100`: `direct_delete=5`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=1/1`, `delete_leaf_compact=5/160ns`,
  `delete_leaf_flush_ns=1973`.
- `fs_delete_1000`: `direct_delete=50`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=6/6`, `delete_leaf_compact=50/1332ns`,
  `delete_leaf_flush_ns=6160`.
- `fs_delete_10000`: `direct_delete=500`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=64/64`, `delete_leaf_active=433/496`,
  `delete_leaf_miss=63`, `delete_leaf_compact=497/12848ns`,
  `delete_leaf_search=560/58219ns`, `delete_leaf_flush_ns=87883`.

## Interpretation

This is a small keep candidate. It directly lowers the compact-shape check
counter for the 10k-row DELETE profile from the prior retained run's
`19272 ns` to `12848 ns`, and the end-to-end 10k-row DELETE row moved from
`363.6 us` to `342.2 us`. It does not close the remaining DELETE gap.
