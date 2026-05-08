# Current DML Setup And Isolated Mutation Profile

## Context

This is a read-only source profiling pass after
`568a337b docs(perf): rerun current write frontier`. Rust source was unchanged
since the clean release-perf binary was built at:

- `/data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete`

The goal was to separate the remaining 100-row UPDATE/DELETE tail from the
already fenced setup fast paths: transaction-control parse bypass, benchmark
PRAGMA skips, schema/root lookup trims, microbatch carry, retained DML cursor
hints, page-cache shard fanout, and direct record/page-builder micro-patches.

## Commands

```bash
perf record -F 999 -g --call-graph dwarf \
  -o tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf.data \
  -- /data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete \
  100 10000 both fsqlite standard

perf report --stdio --no-children \
  -i tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf.data \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-no-children.txt

perf report --stdio \
  -i tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf.data \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-children.txt

/data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete \
  100 10000 both compare standard \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/stdout/perf-update-delete-compare.txt 2>&1

perf record -F 999 -g --call-graph dwarf \
  -o tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-isolated.data \
  -- /data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete \
  100 10000 both fsqlite isolated

perf report --stdio --no-children \
  -i tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-isolated.data \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-isolated-no-children.txt

perf report --stdio \
  -i tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-isolated.data \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/perf-isolated-children.txt

/data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete \
  100 10000 both compare isolated \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/stdout/perf-update-delete-isolated-compare.txt 2>&1

/data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete \
  100 20000 update compare isolated \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/stdout/perf-update-delete-isolated-update-compare.txt 2>&1

/data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/perf-update-delete \
  100 20000 delete compare isolated \
  > tests/artifacts/perf/swiftgate-dml-setup-perf-20260508T1915Z/stdout/perf-update-delete-isolated-delete-compare.txt 2>&1
```

`perf report` emitted the expected restricted kernel symbol warning. User-space
symbols resolved.

## Standard 100-Row DML Profile

Standard mode opens a fresh in-memory connection, applies benchmark PRAGMAs,
creates and populates the table, then runs UPDATE and DELETE per iteration.

```text
fsqlite: total=1767ms populate=312ms update=144ms delete=99ms
per-row-update=1448ns per-row-delete=1987ns
```

The direct comparison run was:

```text
fsqlite: total=1780ms populate=321ms update=148ms delete=97ms
sqlite:  total=831ms  populate=287ms update=44ms  delete=20ms
ratio:   total=2.14x populate=1.12x update=3.34x delete=4.67x
```

Top flat symbols in the standard fsqlite profile:

| Self | Symbol | Interpretation |
| ---: | --- | --- |
| 7.96% | `__memmove_avx_unaligned_erms` | page/record copy and allocator movement |
| 6.38% | `_int_malloc` | setup and page/record allocation |
| 3.84% | `Connection::try_serialize_prepared_direct_simple_insert_record` | populate INSERT record build |
| 1.51% | `Connection::execute_prepared_direct_simple_insert` | populate INSERT path |
| 1.46% | `ShardedPageCache::with_max_buffers_for_initial_pages` | connection/page-cache open setup |
| 1.01% | `SharedMvccState::new` | connection open-state setup |
| 0.99% | `Connection::open_with_env_and_pager` | connection setup |

This mixed profile is dominated by open/populate/setup and therefore does not
justify a transaction-control or page-cache micro-patch. Those families are
already in the negative ledger and the current profile does not newly elevate
one unfenced open-state symbol.

## Isolated Mutation Profile

Isolated mode removes fresh-connection setup from the repeated mutation loop:
UPDATEs run inside one transaction, and DELETEs consume unique prepopulated rows
inside one transaction.

```text
fsqlite: total=195ms populate=14ms update=94ms delete=73ms
per-row-update=945ns per-row-delete=1469ns
```

Mixed isolated comparison:

```text
fsqlite: total=178ms populate=13ms update=86ms delete=66ms
sqlite:  total=63ms  populate=16ms update=32ms delete=13ms
ratio:   total=2.83x populate=0.81x update=2.67x delete=4.83x
```

Update-only isolated comparison:

```text
fsqlite: update=136ms, per-row-update=681ns
sqlite:  update=56ms,  per-row-update=283ns
ratio:   update=2.40x
```

Delete-only isolated comparison:

```text
fsqlite: delete=166ms, per-row-delete=1666ns
sqlite:  delete=30ms,  per-row-delete=301ns
ratio:   delete=5.54x
```

Top flat symbols in the isolated fsqlite profile:

| Self | Symbol | Interpretation |
| ---: | --- | --- |
| 14.70% | `__memmove_avx_unaligned_erms` | page-copy/write and cell movement |
| 7.89% | `BtCursor<SharedTxnPageIo>::table_seek_for_insert` | repeated B-tree positioning |
| 7.66% | `_int_malloc` | transient cursor/page/record allocation |
| 6.14% | `BtCursor<SharedTxnPageIo>::delete` | delete operator body |
| 2.79% | `TransactionKind::write_page_data` | staged page write path |
| 2.53% | `fsqlite_btree::cell::read_cell_pointers_into` | repeated leaf-cell pointer decode |
| 2.48% | `TransactionKind::get_page` | page fetch |
| 2.41% | `BtCursor<SharedTxnPageIo>::load_page` | cursor page load |
| 2.13% | `BtCursor<TransactionPageIo>::local_leaf_table_cell` | current-cell payload access |
| 1.96% | `table_overwrite_current_payload_same_size_no_overflow` closure | same-size UPDATE write |
| 1.66% | `fsqlite_btree::cell::write_cell_pointers` | delete/page mutation |
| 1.53% | `Connection::reload_memdb_from_txn_with_mode` | transaction image refresh |

## Decision

No source patch was attempted in this pass.

The standard profile says the remaining 100-row tail still includes connection
and populate ceremony, but the isolated profile shows the actual mutation gap is
not a single unfenced helper. It is distributed across B-tree seek/delete,
page-copy/write, cell-pointer decoding, and allocator churn. The narrow variants
that map to those symbols have already been rejected or fenced:

- retained DML cursor/seek hints,
- direct UPDATE/DELETE microbatch carry,
- same-size overwrite and fixed-payload patches,
- page-cache shard/open-state allocation trims,
- page-builder/direct record serialization micro-patches,
- transaction-control and benchmark-PRAGMA fast paths.

The next plausible source lever is therefore still the broader DML leaf-run
operator described in earlier frontier artifacts: prove a same-leaf
UPDATE/DELETE run can retain the decoded leaf state and page writer across a
statement burst while preserving read-after-write visibility, rollback/savepoint
boundaries, schema invalidation, and concurrent-mode defaults. A cursor-local
micro-patch without that broader operator is unlikely to move the matrix.
