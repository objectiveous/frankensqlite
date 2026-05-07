# Prepared direct INSERT record-cell layout keep

Date: 2026-05-07 19:50Z
Agent: CrimsonGorge
Source: `ba8e9dae refactor(prepared): stream record-cell layout sizes during column iteration`

## Candidate

The prepared direct INSERT record builder now carries each evaluated value's
`serial_type` and `payload_len` in a `PreparedDirectInsertRecordCell` during the
column iteration. The serializer receives the precomputed header/body sizes and
cell array, removing the previous second layout pass and the parallel
`SmallVec<[(u64, usize); 16]>`.

This is the SQLite `OP_MakeRecord` shape: size each field once, then emit from
the cached per-cell layout. Record order, affinity handling, rowid alias NULL
storage, NaN-to-NULL behavior, and bind-error ordering remain on the existing
direct INSERT path.

## Proof

- `cargo fmt -p fsqlite-core --check` passed.
- Focused direct INSERT tests passed:
  - `test_prepared_direct_simple_insert_numeric_binary_ops_preserve_sqlite_edges`
  - `test_prepared_direct_simple_insert_large_profile_breakdown`
  - `test_prepared_direct_simple_insert_autocommit_profile_breakdown`
- `cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
  passed.
- The broader `prepared_direct_simple_insert` filter has one known pre-existing
  failure: `test_prepared_direct_simple_insert_autocommit_retains_memory_append_hint`.
  It fails on parent `9c54d8f9` as well as on `ba8e9dae`, so it is not caused by
  the record-cell layout change.

## Benchmarks

Commands:

```text
/data/tmp/frankensqlite-direct-insert-layout-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/direct-insert-layout-crimsongorge-20260507T1950Z/insert-layout.json \
  --no-html

/data/tmp/frankensqlite-direct-insert-layout-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/direct-insert-layout-crimsongorge-20260507T1950Z/full-quick-layout.json \
  --no-html

FSQLITE_BENCH_PROFILE_INSERT=1 \
/data/tmp/frankensqlite-direct-insert-layout-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/direct-insert-layout-crimsongorge-20260507T1950Z/insert-layout-profiled.json \
  --no-html
```

Artifacts:

- `insert-layout.json`
- `full-quick-layout.json`
- `insert-layout-profiled.json`
- `stdout/`

Compared with the prior post-numeric full quick artifact
`tests/artifacts/perf/full-quick-insert-numeric-repeat-crimsongorge-20260507T2025Z/full-quick-insert-numeric-repeat.json`:

| Metric | Before | After |
| --- | ---: | ---: |
| Primary score | 0.35304311937129634 | 0.3445386401431955 |
| Average ratio | 0.46648821322957423 | 0.45557973340836866 |
| Geomean ratio | 0.26988400673068275 | 0.2635206749084158 |
| Franken faster rows | 78 | 79 |
| Comparable rows | 8 | 5 |
| C SQLite faster rows | 7 | 9 |

The primary score and geomean moved in the right direction, so this passes the
matrix keep gate. The full quick p90/p99 worsened because the noisy
UPDATE/DELETE rows moved up in this run; those rows are outside the touched
code path and already have multiple rejected standalone candidates in the
negative-results ledger.

## Remaining strict gaps

Top remaining rows in `full-quick-layout.json` are still:

- `100 rows / delete 5 rows`: ratio `1.4470359808264062`
- `100 rows / update 10 rows`: ratio `1.3849202643674583`
- Small 100-row INSERT variants: ratios about `1.08` to `1.15`
- `2 writers x 1000 rows`: ratio `1.044202199755874`

The next high-EV lane is still either a true direct UPDATE/DELETE mutation
primitive change, or another direct INSERT row-builder reduction that directly
attacks the remaining 100-row fixed overhead.
