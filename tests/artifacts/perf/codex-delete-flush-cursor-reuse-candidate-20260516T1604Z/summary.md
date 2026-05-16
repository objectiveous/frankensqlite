# Direct DELETE Leaf-Flush Cursor Reuse Candidate

- Date: 2026-05-16
- Source: local tree based on `cb5ac19d5490881603e07d364c6b8ad17475f65d`
- Change under test: reuse one B-tree cursor while flushing pending direct
  DELETE leaf runs that share the same root page, page size, and reserved-byte
  header shape.
- Command:
  `rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-delete-flush-cursor-reuse-candidate-20260516T1604Z/update-delete.json --no-html`
- Note: the benchmark reported a JSON output path, but local RCH retrieval left
  only an ignored empty `run.log`. This summary preserves the terminal output
  rows and counters.

## Result

Compared with the retained head DML profile in
`tests/artifacts/perf/codex-head-dml-profile-1e1abcd5-20260516T130048Z/summary.md`,
FrankenSQLite DELETE absolute time improved on all quick update-delete DELETE
rows. The 10k-row DELETE path moved from `451.3 us` to `363.6 us`, and the
profiled leaf-run flush time moved from `91483 ns` to `60140 ns`.

| Scenario | C SQLite | FrankenSQLite | Ratio | Status |
|---|---:|---:|---:|---|
| `100 rows / update 10 rows` | `4.5 us` | `6.0 us` | `1.33x` | slower |
| `100 rows / delete 5 rows` | `2.5 us` | `6.2 us` | `2.55x` | slower |
| `1000 rows / update 100 rows` | `42.7 us` | `31.4 us` | `1.36x` | faster |
| `1000 rows / delete 50 rows` | `18.2 us` | `31.7 us` | `1.74x` | slower |
| `10000 rows / update 1000 rows` | `306.9 us` | `225.1 us` | `1.36x` | faster |
| `10000 rows / delete 500 rows` | `113.6 us` | `363.6 us` | `3.20x` | slower |

Summary statistics from the run: `6` scenarios, `2` FrankenSQLite-faster,
`0` comparable, `4` C-SQLite-faster, average time ratio `1.71x`.

## Key Counters

The DELETE path stayed on the direct prepared path:

- `fs_delete_100`: `direct_delete=5`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=1/1`, `delete_leaf_flush_ns=1623`,
  `delete_leaf_search=5/801ns`.
- `fs_delete_1000`: `direct_delete=50`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=6/6`, `delete_leaf_flush_ns=7682`,
  `delete_leaf_search=55/6468ns`.
- `fs_delete_10000`: `direct_delete=500`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=64/64`, `delete_leaf_active=433/496`,
  `delete_leaf_miss=63`, `delete_leaf_materialize=64/44747ns`,
  `delete_leaf_search=560/50503ns`, `delete_leaf_flush_ns=60140`.

## Interpretation

This is a keep candidate, not a full gap closure. It removes repeated cursor
setup from a measured direct DELETE flush path and lowers the internal flush
counter, but the benchmark still shows DELETE slower than C SQLite at every
row count.
