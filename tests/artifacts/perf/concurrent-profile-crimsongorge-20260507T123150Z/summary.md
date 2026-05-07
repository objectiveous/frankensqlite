# Concurrent/MVCC profile after staged-delete rejection

- Agent: CrimsonGorge
- Source commit: `5b36871d3fb29b9728ec9e34c64a8c21b45151f1`
- Clean profiling worktree: `/data/tmp/frankensqlite-concurrent-profile-crimsongorge-20260507T123150Z`
- Build target: `/data/tmp/frankensqlite-concurrent-profile-target`
- Build command:
  `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-concurrent-profile-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin mt-mvcc-bench --bin comprehensive-bench --profile release-perf`

## Matrix anchor

`comprehensive-bench --quick --filter concurrent`:

| Scenario | C SQLite | FrankenSQLite | Ratio |
|---|---:|---:|---:|
| 2 writers x 1000 rows | 12.85 ms | 13.63 ms | 1.06x slower |
| 4 writers x 1000 rows | 19.44 ms | 20.15 ms | 1.04x comparable |
| 8 writers x 1000 rows | 91.00 ms | 34.96 ms | 2.60x faster |

The comprehensive concurrent row is not a broad concurrency collapse. The only
remaining visible deficit is the low-writer fixed/per-row cost.

## Dedicated mt-mvcc sweep

`mt-mvcc-bench --threads=2,4,8 --rows-per-thread=1000 --iters=5`:

| Threads | FSQLite p50 | C SQLite p50 | Time ratio | Throughput ratio |
|---:|---:|---:|---:|---:|
| 2 | 4.59 ms | 2.12 ms | 2.17x | 0.46x |
| 4 | 11.50 ms | 9.80 ms | 1.17x | 0.85x |
| 8 | 28.51 ms | 80.90 ms | 0.35x | 2.84x |

`--apples-to-apples` is a compatibility flag in this binary and produced the
same shape: 2t `1.80x`, 4t `1.06x`, 8t `0.35x`.

## Setup isolation

`mt-mvcc-bench --rows-per-thread=0 --threads=2,4,8 --iters=10`:

| Threads | FSQLite p50 | C SQLite p50 | Time ratio |
|---:|---:|---:|---:|
| 2 | 0.64 ms | 0.47 ms | 1.36x |
| 4 | 0.84 ms | 1.01 ms | 0.83x |
| 8 | 1.37 ms | 1.16 ms | 1.18x |

Open/start cost explains only about 0.17 ms of the 2-thread deficit. The 2t
write rows show the larger slope:

| Rows/thread | FSQLite p50 | C SQLite p50 | Time ratio |
|---:|---:|---:|---:|
| 1 | 1.94 ms | 1.60 ms | 1.21x |
| 10 | 1.95 ms | 1.60 ms | 1.21x |
| 100 | 2.03 ms | 1.69 ms | 1.20x |
| 1000 | 4.59 ms | 2.12 ms | 2.17x |

## Perf attribution

`perf record -F 1999 --call-graph fp` over the 2-thread mt-mvcc run captured
2,957 samples. Top resolved self-time entries were:

- `__memmove_avx_unaligned_erms`: `4.31%`
- `Connection::execute_prepared_direct_simple_insert`: `2.54%`
- `sqlite3VdbeExec`: `2.32%`
- `_int_malloc`: `2.05%`
- `fsqlite_btree::cell::read_cell_pointers_into`: `1.67%`
- `alloc::fmt::format::format_inner`: `1.38%`
- `BtCursor<SharedTxnPageIo>::load_page`: `1.37%`
- `Connection::refresh_eprocess_oracle`: `0.99%`
- `Connection::execute_prepared_direct_simple_insert_with_cursor`: `0.98%`
- `Connection::finish_prepared_direct_simple_insert_after_storage`: `0.87%`

This points back to direct INSERT row construction plus btree append/seek/page
layout, not to WAL/MVCC commit validation as the dominant residual. The
high-EV next candidate is still a real non-empty-root bulk/right-edge page
builder or fused record-body plus page-layout builder. That seam currently
requires `crates/fsqlite-core/src/connection.rs`, which was dirty and reserved
by PurpleOtter during this pass. I did not edit through that lock.

Low-level btree/type alternatives were checked against the negative ledger:
standalone `write_varint`, rightmost-hint ownership, staged rightmost hints,
cell-pointer pooling, and append-hint-started page-run replay have already
been rejected or fenced unless they are part of a broader retained page-builder
design. No source candidate from this pass cleared the evidence bar.
