# Storage Rowid DELETE Buffer Reject

Date: 2026-05-11
Source state tested: uncommitted `Connection` prototype, later reverted

## Candidate

Prototype an exact storage-rowid membership manifest for private `:memory:`
tables, then use it to buffer prepared direct DELETE rowids logically until the
next read/commit boundary. The goal was to avoid per-row leaf-run misses across
DELETE batches without hydrating full MemDatabase row payloads.

The prototype added focused tests for read-boundary flushes, duplicate/absent
rowids, rollback, and missing-active-transaction restore. It also exposed and
fixed one restore bug in the rowid-run flush path before measurement.

## Correctness Checks

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rowid-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core pending_direct_delete -- --nocapture --test-threads=1
result: pass, 6 tests

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rowid-local CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core prepared_direct_delete -- --nocapture --test-threads=1
result: pass, 8 tests
```

## Focused Benchmark

Binary:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rowid-local CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete
```

Measured with `perf-update-delete <rows> 20 delete compare standard`.

| Rows | Deletes | FrankenSQLite per delete | C SQLite per delete | Delete ratio |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 5 | 1976 ns | 431 ns | 4.58x |
| 1000 | 50 | 1135 ns | 324 ns | 3.50x |
| 10000 | 500 | 941 ns | 332 ns | 2.83x |

Raw outputs:

- `perf-update-delete-100.txt`
- `perf-update-delete-1000.txt`
- `perf-update-delete-10000.txt`

## Decision

Rejected and reverted. The first-consult membership seed scans the B-tree and
the deferred flush still pays physical delete work, so the measured DELETE rows
move in the wrong direction even though the prototype can be made correct.

Do not retry a build-on-first-DELETE exact rowid membership buffer. Reconsider
only if the manifest is created essentially for free during a proven-empty table
population path, or as part of the broader transaction-local DML mutation
operator that also covers INSERT/UPDATE publication and read-boundary semantics.
