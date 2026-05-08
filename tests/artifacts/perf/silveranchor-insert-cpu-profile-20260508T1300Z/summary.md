# SilverAnchor Insert CPU Profile - 2026-05-08 13:00Z

## Scope

- Clean detached worktree: `/data/tmp/frankensqlite-silveranchor-pager-fix-0382ee26`
- Code baseline: `0382ee26` (`ac57295b` only adds perf docs/artifacts on top)
- Binary: `/data/tmp/frankensqlite-silveranchor-insert-profile-target/release-perf/comprehensive-bench`
- Command:
  `FSQLITE_BENCH_PROFILE_INSERT=1 perf record -F 997 -g --call-graph dwarf -o perf-insert.data -- comprehensive-bench --quick --filter insert --json-out insert-profile.json --no-html`

## Result

The insert-only repeat reproduced a narrow 100-row insert tail, not a stable
large-row gap. The worst insert rows were:

- `small_3col` 100-row batched: `1.145953x` (`0.074339 ms` C SQLite, `0.085189 ms` FrankenSQLite)
- `small_3col` 100-row single-txn strategy: `1.097807x` (`0.075802 ms`, `0.083216 ms`)
- `small_3col` 100-row single-transaction section: `1.096797x` (`0.077936 ms`, `0.085480 ms`)
- `large_10col` 10K record-size row: `1.081145x` (`9.924670 ms`, `10.730008 ms`)

The large 10-column row remains noisy: this repeat showed it slower, while the
earlier same-day insert profile had it faster than C SQLite.

## CPU Profile

Top no-children samples from `perf-insert-report.txt`:

- C SQLite: `sqlite3VdbeExec` `13.10%`, `sqlite3BtreeTableMoveto` `3.04%`, `sqlite3BtreeInsert` `1.80%`
- Shared libc: `__memmove_avx_unaligned_erms` `8.27%`, `__memset_avx2_unaligned_erms` `3.18%`
- FrankenSQLite: `Connection::try_serialize_prepared_direct_simple_insert_record` `7.72%`
- FrankenSQLite: `Connection::execute_prepared_direct_simple_insert` `2.27%`
- FrankenSQLite: `Connection::eval_prepared_direct_simple_insert_expr` `2.21%`
- FrankenSQLite: `fsqlite_types::serial_type::write_varint` `0.75%`

The remaining insert hotspot is therefore inside the direct INSERT serializer
and expression/value construction in `crates/fsqlite-core/src/connection.rs`.
The pager/WAL contribution in this profile is below the threshold for a
credible standalone unreserved change.

## Blocker

`crates/fsqlite-core/src/connection.rs` is currently reserved by `RusticGrove`
until `2026-05-08T13:27:53Z`, together with
`crates/fsqlite-btree/src/cursor.rs`. The active dirty diff is a fixed-width
REAL update leaf-byte patch in that reserved surface, so this pass did not edit
source files.

Any next insert optimization should wait for that reservation or explicitly
coordinate ownership, then target the direct serializer as a broader
row-template/page-run encoder. A standalone `write_varint` or pager micro-patch
is not supported by this profile.
