# Current DML Compare Profile - 2026-05-08

## Source Basis

- Clean-source checkout: `/data/projects/frankensqlite-rusticgrove-clean-next-20260508T1630Z`
- Clean-source Git HEAD: `f749770c test(harness): accept convoy transient update conflicts`
- Main worktree Git HEAD: `1496ac87 docs(perf): record dirty pagebuilder eval`
- Source equivalence: current `HEAD` differs from the clean checkout only by
  committed perf artifacts.
- Build target: `/data/tmp/frankensqlite-rusticgrove-clean-dml-target`

An initial capture in `/data/projects/frankensqlite` found an uncommitted
`crates/fsqlite-core/src/connection.rs` diff afterward, so the clean evidence in
this artifact was rebuilt and rerun from the detached clean checkout above. The
uncommitted main-worktree source diff was not included in these clean
measurements.

This pass refreshed the 100-row `UPDATE/DELETEThroughput` mutation profile after
the page-builder dirty candidate was rejected. It did not attempt a source
patch; the goal was to decide whether the current 100-row DML gap points outside
the already-fenced negative-ledger surfaces.

## Build

```bash
mkdir -p /data/tmp/frankensqlite-rusticgrove-clean-dml-target
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-clean-dml-target \
  CARGO_BUILD_JOBS=12 \
  cargo build --profile release-perf \
  -p fsqlite-e2e \
  --bin perf-update-delete
```

Result: passed.

The shared `/data/tmp/cargo-target` build was first attempted, but the target
directory disappeared during compilation and Cargo reported missing bytecode and
temporary-file paths. The isolated target directory above avoided the shared
cache race.

## Compare Runs

```bash
/data/tmp/frankensqlite-rusticgrove-clean-dml-target/release-perf/perf-update-delete \
  100 20000 update compare isolated
```

Result:

- FSQLite update: `629 ns/row`
- C SQLite update: `280 ns/row`
- Update ratio: `2.25x`

```bash
/data/tmp/frankensqlite-rusticgrove-clean-dml-target/release-perf/perf-update-delete \
  100 20000 delete compare isolated
```

Result:

- FSQLite delete: `1705 ns/row`
- C SQLite delete: `282 ns/row`
- Delete ratio: `6.06x`

```bash
/data/tmp/frankensqlite-rusticgrove-clean-dml-target/release-perf/perf-update-delete \
  100 2000 update compare standard
```

Result:

- FSQLite update: `1570 ns/row`
- C SQLite update: `425 ns/row`
- Update ratio: `3.69x`
- Populate ratio: `0.95x`

```bash
/data/tmp/frankensqlite-rusticgrove-clean-dml-target/release-perf/perf-update-delete \
  100 2000 delete compare standard
```

Result:

- FSQLite delete: `2287 ns/row`
- C SQLite delete: `425 ns/row`
- Delete ratio: `5.38x`
- Populate ratio: `1.02x`

## Flat Profiles

The `perf.data` files were generated locally but are not intended as committed
artifacts. The committed profiler evidence is the text reports:

- `clean-perf-update-100-flat.txt`
- `clean-perf-delete-100-flat.txt`

The profiler emitted the expected restricted-kernel-symbol warning because
`/proc/sys/kernel/kptr_restrict` prevents kernel symbol resolution. User-space
symbols were still resolved.

Top current update self-time:

| Self | Symbol |
| ---: | --- |
| `9.26%` | `__memmove_avx_unaligned_erms` |
| `4.87%` | `Connection::execute_prepared_direct_simple_update` |
| `4.19%` | `_int_malloc` |
| `3.19%` | `BtCursor<SharedTxnPageIo>::table_seek_for_insert` |
| `2.95%` | `SharedTxnPageIo::write_page_internal` |
| `2.94%` | `drop_glue<BtCursor<SharedTxnPageIo>>` |
| `2.89%` | `SharedTxnPageIo::read_page_data` |
| `2.29%` | `BtCursor<SharedTxnPageIo>::load_page` |
| `2.26%` | `BtCursor<TransactionPageIo>::table_leaf_rowid_at` |
| `2.24%` | `parse_record_projected_column_offsets` |

Top current delete self-time:

| Self | Symbol |
| ---: | --- |
| `40.22%` | `TransactionKind::get_page` |
| `13.89%` | `TransactionKind::write_page_data` |
| `4.49%` | `BtCursor<SharedTxnPageIo>::delete` |
| `4.17%` | `__memmove_avx_unaligned_erms` |
| `3.60%` | `BtCursor<SharedTxnPageIo>::table_seek_for_insert` |
| `2.54%` | `_int_malloc` |
| `2.10%` | `read_cell_pointers_into` |
| `1.35%` | `Connection::execute_prepared_direct_simple_delete` |
| `1.14%` | `try_serialize_prepared_direct_simple_insert_record` |
| `1.07%` | `CellRef::parse` |

## Negative-Ledger Fence

The current profile points back into surfaces that already have measured
standalone rejects:

- Same-size UPDATE staged-page overwrite probing.
- Fixed-width REAL payload-range and leaf-local field patching.
- Direct REAL assignment shortcut.
- Direct UPDATE lazy row-scratch borrow.
- Retained direct UPDATE/DELETE cursor shell and retained seek hints.
- Private-memory `SharedTxnPageIo` bypass.
- Reusable `SharedTxnPageIo` shell.
- Staged table-leaf DELETE mutation before clone fallback.
- Pending direct DELETE leaf-run via repeated seeks.
- Direct DELETE no-rebalance leaf primitive.
- Tier0 already-staged marker skip.
- `SimpleTransaction::get_page` staged-publication split.

## Decision

No source patch was attempted.

The fresh profile confirms the 100-row DML tail is a real mutation-kernel gap
against C SQLite, especially DELETE, but the top symbols are not a new
unfenced lever. The next plausible source contract remains a larger B-tree or
pager redesign that removes root descent plus page read/write ceremony together,
not another one-symbol branch, cache, cursor shell, or payload patch.

Current keep gate for any future DML candidate:

1. Prove an isolated `perf-update-delete 100 ... compare isolated` win in
   absolute FSQLite ns/row for both UPDATE and DELETE or explicitly explain why
   the untouched half cannot regress.
2. Pass repeated `comprehensive-bench --quick --filter update` gates.
3. Promote to full quick only if the focused gate improves average, geomean,
   P90/P99, and the 100-row tails in the same A/B window.
