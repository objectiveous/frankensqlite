# Explicit Transaction Append-Hint Leaf Image Retention

Date: 2026-04-27
Agent: SnowyFortress
Base commit: `ffd0a4c0495e578a25c6e9920d57ff6ac823b4db`
Candidate: retain the prepared direct-insert append hint's cached right-edge leaf image while an explicit transaction is active.

## Profile Signal

The current `perf-update-delete 10000 100 both` profile still spends its largest CPU share in populate-phase direct inserts:

- `__memmove_avx_unaligned_erms`: 9.35%
- Stack includes `try_append_table_leaf_payload_in_place_no_overflow` -> `try_table_append_on_hinted_leaf_with_known_last_rowid` -> `execute_prepared_direct_simple_insert`

The benchmark repopulates the table inside `BEGIN`/`COMMIT` for every outer iteration, so this insert-side page motion is part of the measured hot path even when the named workload is update/delete.

## Change

Before this candidate, `store_prepared_direct_insert_append_hint` stripped `cached_leaf.page_data` for explicit transactions and only retained it for the retained memory-autocommit path. The direct insert path already knows how to reuse a cached leaf image safely inside explicit transaction tests, so this candidate extends retention to `self.in_transaction.get()`.

No concurrent-writer defaults or file-locking behavior changed.

## Measurements

Commands used `release-perf` binaries built with:

```bash
CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only \
CARGO_PROFILE_RELEASE_PERF_STRIP=false \
RUSTFLAGS='-C force-frame-pointers=yes' \
cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Hyperfine results:

| Workload | Base | Candidate | Result |
| --- | ---: | ---: | ---: |
| `10000 100 both` | 1.661s +/- 0.030s | 1.567s +/- 0.026s | 1.06x +/- 0.03 faster |
| `10000 100 update` | 1.442s +/- 0.010s | 1.347s +/- 0.035s | 1.07x +/- 0.03 faster |
| `10000 100 delete` | 1.098s +/- 0.019s | 0.990s +/- 0.012s | 1.11x +/- 0.02 faster |

JSON exports:

- `hyperfine-10000x100-both.json`
- `hyperfine-10000x100-update.json`
- `hyperfine-10000x100-delete.json`

One illustrative phase-timing run for `10000 100 both`:

| Binary | Total | Populate | Update | Delete |
| --- | ---: | ---: | ---: | ---: |
| Base | 1625ms | 839ms | 482ms | 232ms |
| Candidate | 1549ms | 742ms | 476ms | 255ms |

The phase timer is a single run and is included only to confirm that the main measured win lands in the explicit populate transaction.

## Verification

Passed:

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-explicit-hint-check cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-explicit-hint-check cargo clippy --workspace --all-targets -- -D warnings
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260427-explicit-hint-test cargo test -p fsqlite-core test_prepared_direct_simple_insert -- --nocapture --test-threads=1 --skip test_prepared_direct_simple_insert_autocommit_profile_breakdown --skip test_prepared_direct_simple_insert_memory_missing_memdb_table_uses_autocommit_fast_path
```

The filtered test command ran 22 prepared-direct-insert tests successfully, including explicit transaction, savepoint rollback, and other-table-write hint reuse coverage.

Known pre-existing failures reproduced on clean base commit `ffd0a4c0495e578a25c6e9920d57ff6ac823b4db` and on the candidate:

- `test_prepared_direct_simple_insert_autocommit_profile_breakdown`: `BusySnapshot` on pages 1..54
- `test_prepared_direct_simple_insert_memory_missing_memdb_table_uses_autocommit_fast_path`: expected one direct insert execution, observed slow path execution

UBS note:

- `timeout 180s ubs crates/fsqlite-core/src/connection.rs crates/fsqlite-harness/src/bin/spec_to_beads_audit.rs` timed out in the Rust ast-grep phase.
- `timeout 60s ubs crates/fsqlite-harness/src/bin/spec_to_beads_audit.rs` completed with 0 critical issues and 4 pre-existing warnings about allocation in argument parsing.
