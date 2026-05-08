# Setup vs Mutation Review

Date: 2026-05-08
Agent: SilverAnchor
Source commit: `3373bb4dc7fe2dbdf1d5c7571fe08e1dd6ae3786`

## Scope

This pass refreshed the remaining `UPDATE/DELETEThroughput` tail after the
current full-frontier repeat. The goal was to decide whether the remaining
100-row rows are setup/populate artifacts or a real direct-DML mutation gap.

The checkout already contained uncommitted edits in:

- `crates/fsqlite-core/src/connection.rs`
- `crates/fsqlite-btree/src/cursor.rs`

Agent Mail showed both files are exclusively reserved by `WindyIbis` until
`2026-05-08T12:36:04Z`, so no source edits were made in this pass.

## Build

Command:

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-next-target CARGO_BUILD_JOBS=12 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf
```

The remote compile succeeded. Artifact retrieval took `344.6s` because `rch`
mirrored a large target tree, but the benchmark binaries were present at:

```text
/data/tmp/frankensqlite-boldlion-next-target/release-perf/comprehensive-bench
/data/tmp/frankensqlite-boldlion-next-target/release-perf/perf-update-delete
```

The benchmark reported a binary-predates-HEAD warning. HEAD contains recent
docs/artifact commits, and the current source candidate in the shared checkout
was not used as a keep result.

## Focused Section 6 Repeat

Command:

```text
env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-boldlion-next-target/release-perf/comprehensive-bench --quick --filter update --no-html
```

Rows:

| Row | Ratio | C SQLite | FrankenSQLite | F CV% |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | `1.29x` | `89.9 us` | `115.6 us` | `5.1` |
| 100 rows / delete 5 rows | `1.32x` | `82.9 us` | `109.2 us` | `4.0` |
| 1000 rows / update 100 rows | `1.05x` | `410.2 us` | `391.3 us` | `2.4` |
| 1000 rows / delete 50 rows | `1.05x` | `375.8 us` | `357.5 us` | `2.9` |
| 10000 rows / update 1000 rows | `1.04x` | `3.68 ms` | `3.55 ms` | `8.5` |
| 10000 rows / delete 500 rows | `1.04x` | `3.43 ms` | `3.31 ms` | `6.3` |

This repeat softened the prior DML spike: only the 100-row rows remain slower,
while 1K and 10K are comparable/faster.

The FSQLite profile still shows all rows on the direct fast path:
`fast == mutations`, `slow == 0`, `vdbe_opcodes == 0`, and
`vdbe_statements == 0`.

## Mutation Split

Command:

```text
/data/tmp/frankensqlite-boldlion-next-target/release-perf/perf-update-delete 100 20000 both compare standard
```

Result:

```text
fsqlite: total=3301ms populate=554ms update=278ms delete=187ms | per-row-update=1391ns per-row-delete=1876ns
sqlite:  total=1678ms populate=606ms update=88ms  delete=41ms  | per-row-update=440ns  per-row-delete=411ns
ratio:   total=1.97x populate=0.91x update=3.16x delete=4.57x
```

Command:

```text
/data/tmp/frankensqlite-boldlion-next-target/release-perf/perf-update-delete 100 20000 both compare isolated
```

Result:

```text
fsqlite: total=418ms populate=27ms update=192ms delete=175ms | per-row-update=960ns per-row-delete=1754ns
sqlite:  total=144ms populate=38ms update=74ms  delete=31ms  | per-row-update=370ns per-row-delete=313ns
ratio:   total=2.89x populate=0.72x update=2.59x delete=5.60x
```

Conclusion: the remaining 100-row Section 6 tail is not caused by population.
FSQLite population is faster in this focused harness. The gap is a real
direct-DML mutation-kernel gap, especially DELETE, but the absolute full-row
Section 6 delta is now about `25-30 us`.

## Delete CPU Profile

Command:

```text
perf record -F 999 -g --call-graph dwarf -o tests/artifacts/perf/boldlion-setup-mutation-review-20260508T1040Z/perf-delete-100-isolated.data -- /data/tmp/frankensqlite-boldlion-next-target/release-perf/perf-update-delete 100 40000 delete fsqlite isolated
perf report --stdio --no-children --sort comm,dso,symbol -i tests/artifacts/perf/boldlion-setup-mutation-review-20260508T1040Z/perf-delete-100-isolated.data
```

Top flat self-time symbols:

| Self | Symbol |
| ---: | --- |
| `24.09%` | `TransactionKind::get_page` |
| `9.41%` | `TransactionKind::write_page_data` |
| `5.96%` | `__memmove_avx_unaligned_erms` |
| `5.86%` | `BtCursor<SharedTxnPageIo>::table_seek_for_insert` |
| `5.83%` | `BtCursor<SharedTxnPageIo> as BtreeCursorOps>::delete` |
| `4.20%` | `_int_malloc` |
| `2.22%` | `Connection::try_serialize_prepared_direct_simple_insert_record` |
| `2.15%` | `read_cell_pointers_into` |
| `1.92%` | `BtCursor<SharedTxnPageIo>::load_page` |
| `1.59%` | `SharedTxnPageIo as PageReader>::read_page_data` |

This matches the previous delete-tail profile. The top frames are still the
page I/O and table-leaf DELETE ceremony already covered by the negative-results
ledger as standalone microprobes.

## Candidate Guidance

The dirty reserved source already appears to be a true leaf-local DELETE run
candidate: `BtCursor::table_delete_current_leaf_rowids_no_rebalance` plus
pending `Connection` state. That is the right family to test next, but it must
be finished by the reservation holder or coordinated explicitly.

Do not spend another pass on the already-rejected standalone variants:

- private-memory `SharedTxnPageIo` bypass,
- retained direct-DML cursor shell,
- retained table-seek hints,
- non-max/no-rebalance single-row DELETE primitive,
- staged-page table-leaf mutation,
- top-stack clone removal,
- single-freeblock DELETE,
- scratch/lookaside/QF microprobes.

The next keep gate for the dirty leaf-run candidate should be:

1. `cargo test -p fsqlite-btree table_delete cursor_delete insert_delete -- --nocapture`
2. focused direct DELETE correctness in `fsqlite-core`
3. isolated `perf-update-delete 100/1000/10000 delete compare isolated`
4. `comprehensive-bench --quick --filter update --no-html`
5. full quick matrix only if the focused gate wins.

## Artifacts

- `stdout/update-profile.out`
- `stdout/update-profile.err`
- `stdout/perf-update-delete-100-compare.out`
- `stdout/perf-update-delete-100-compare.err`
- `stdout/perf-update-delete-100-isolated-compare.out`
- `stdout/perf-update-delete-100-isolated-compare.err`
- `stdout/perf-delete-100-isolated.out`
- `stdout/perf-delete-100-isolated.err`
- `stdout/perf-delete-100-isolated-report.txt`
- `perf-delete-100-isolated.data`
