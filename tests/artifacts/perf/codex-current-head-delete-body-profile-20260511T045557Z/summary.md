# Current DELETE Body Frontier

Measured commit: `be7523b923de4bc49cc9d421ee451c1d298700c1`
Measured date: 2026-05-11T04:55:57Z

## Results

| workload | FSQLite per-row DELETE | C SQLite per-row DELETE | ratio |
| --- | ---: | ---: | ---: |
| 1000 rows, 30 iters, standard | 715 ns | 322 ns | 2.22x |
| 1000 rows, 30 iters, isolated | 305 ns | 250 ns | 1.22x |
| 1000 rows, 30 iters, rollback-isolated | 272 ns | 286 ns | 0.95x |
| 10000 rows, 10 iters, standard | 621 ns | 326 ns | 1.90x |
| 10000 rows, 10 iters, isolated | 358 ns | 269 ns | 1.33x |
| 10000 rows, 10 iters, rollback-isolated | 521 ns | 302 ns | 1.72x |

## Interpretation

The DELETE body is not the remaining 2x cliff by itself. At 1000 rows the
isolated body is only 1.22x C SQLite, and rollback-isolated body timing is
slightly faster than C SQLite. The standard workload remains 2.22x at 1000 rows
and 1.90x at 10000 rows because the benchmark pays the transaction publication
and pending-delete materialization boundary once per standard iteration.

The standard perf sample is intentionally retained but is populate-heavy:
`try_serialize_prepared_direct_simple_insert_record` is the top self-time
symbol. The isolated perf sample is a better DELETE-path hint, but it still
contains setup/populate samples because the harness must create enough rows for
the isolated delete stream. Its top DELETE-relevant entries are
`TableLeafDeleteRun::delete_rowid_with_reason`, balance/child replacement, page
copy/move, and `execute_prepared_with_params_after_background_status`.

No source patch was kept from this pass. The current evidence continues to
point at the broader `bd-db300.11.1` transaction-local DML mutation operator:
buffer DELETE/UPDATE intent at the transaction boundary and publish it through
the same MVCC/write-witness surface, instead of spending another standalone
patch on leaf-run admission, direct-flush wrappers, tombstone-only overlays,
dense-rowid queues, retained cursors, or PageData move tweaks.
