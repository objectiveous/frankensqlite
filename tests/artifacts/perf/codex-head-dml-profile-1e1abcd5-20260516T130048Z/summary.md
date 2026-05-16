# Current DML Profile for Engine Source 1e1abcd5

- Date: 2026-05-16
- Source: `1e1abcd51d4bf72d215ebb3b1ca7a80a724dc914`
- Scope: production engine source at the benchmarked commit; later local
  commits only added tests, documentation, and retained artifacts.
- Command: `rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-head-dml-profile-1e1abcd5-20260516T130048Z/update-delete.json --no-html`
- Raw local log: `tests/artifacts/perf/codex-head-dml-profile-1e1abcd5-20260516T130048Z/run.log`
- Note: the benchmark reported `update-delete.json`, but the JSON report was
  not present after RCH artifact retrieval. This summary preserves the key rows
  and counters from the retained raw log.

## Result

The benchmarked engine source still has the same update-delete frontier: UPDATE
is already faster at 1k/10k rows, but DELETE remains slower than C SQLite.

| Scenario | C SQLite | FrankenSQLite | Ratio | Status |
|---|---:|---:|---:|---|
| `100 rows / update 10 rows` | `6.1 us` | `8.8 us` | `1.44x` | slower |
| `100 rows / delete 5 rows` | `3.4 us` | `9.4 us` | `2.78x` | slower |
| `1000 rows / update 100 rows` | `54.1 us` | `44.1 us` | `1.23x` | faster |
| `1000 rows / delete 50 rows` | `23.9 us` | `44.4 us` | `1.85x` | slower |
| `10000 rows / update 1000 rows` | `550.7 us` | `399.0 us` | `1.38x` | faster |
| `10000 rows / delete 500 rows` | `243.1 us` | `451.3 us` | `1.86x` | slower |

Summary statistics from the run: `6` scenarios, `2` FrankenSQLite-faster,
`0` comparable, `4` C-SQLite-faster, average time ratio `1.58x`.

## Key Counters

The profile confirms DELETE is still on the prepared direct path, not falling
back to VDBE:

- `fs_delete_100`: `direct_delete=5`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=1/1`, `delete_leaf_search=5/831ns`.
- `fs_delete_1000`: `direct_delete=50`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=6/6`, `delete_leaf_search=55/6885ns`.
- `fs_delete_10000`: `direct_delete=500`, `slow=0`, `vdbe_opcodes=0`,
  `delete_leaf_flush=64/64`, `delete_leaf_active=433/496`,
  `delete_leaf_miss=63`, `delete_leaf_materialize=64/69322ns`,
  `delete_leaf_search=560/59623ns`, `delete_leaf_flush_ns=91483`.

## Interpretation

This supports the existing negative-results boundary: the remaining DELETE gap
is not a missed direct-path admission problem. Another standalone retained
leaf-run tweak is unlikely to close the row; the viable next lever remains a
transaction-local DML mutation operator with grouped leaf/range flush.
