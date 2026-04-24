# 1-thread MVCC gap profile (2026-04-24T20:50Z)

**HEAD at capture:** `03c4988612cd4ed4bcc294f434fdcec1c9df0c4e`
**Binary:** `cargo build --profile=release-perf -p fsqlite-e2e --bin mt-mvcc-bench`
**Command:**
```
perf record -F 999 --call-graph=dwarf,16384 -o perf.data -- \
  mt-mvcc-bench --rows-per-thread=2000 --threads=1 --iters=15 \
  --apples-to-apples
```
**Bench result at capture:** fs_wps = 657,415 / sq_wps = 1,344,216 → 0.49× ratio
(p50 3.04 ms vs 1.49 ms; time_ratio 2.04×).

## Top-10 hot symbols (self-time, 1-thread fsqlite INSERT workload)

Raw listing in `perf-top20.txt`. Symbol attribution marked below:

| Rank | Symbol | Self-time | Side | Interpretation |
|-----:|---|---:|:-:|---|
|   1 | `cfree@GLIBC_2.2.5`                                             | 4.29% | shared | allocator free — both sides contribute |
|   2 | `Connection::store_prepared_direct_insert_append_hint`          | 3.98% | **fsqlite** | per-INSERT hint storage (this commit) |
|   3 | `fsqlite_btree::cell::read_cell_pointers_into`                  | 3.78% | **fsqlite** | btree cell parsing |
|   4 | `_int_malloc`                                                   | 3.65% | shared | allocator malloc |
|   5 | `Connection::execute_prepared_direct_simple_insert`             | 3.62% | **fsqlite** | fast-path INSERT dispatch |
|   6 | `sqlite3VdbeExec`                                               | 3.38% | sqlite | rusqlite's VDBE exec — C side |
|   7 | `<std::path::Path>::_starts_with`                               | 2.74% | fsqlite | path guards in connection open/utility |
|   8 | `<std::sys::pal::unix::time::Timespec>::sub_timespec`           | 2.66% | shared | `Instant::now()` arithmetic |
|   9 | `<usize>::_fmt_inner`                                           | 2.41% | shared | integer formatting (tracing / logs) |
|  10 | `sqlite3MemRealloc`                                             | 2.41% | sqlite | rusqlite's realloc — C side |

## Pure-MVCC top 5 (fsqlite-only, excluding allocator + sqlite-side)

Filtering to fsqlite-attributable symbols only:

| Rank | Symbol | Self-time |
|-----:|---|---:|
|   1 | `Connection::store_prepared_direct_insert_append_hint`    | 3.98% |
|   2 | `fsqlite_btree::cell::read_cell_pointers_into`            | 3.78% |
|   3 | `Connection::execute_prepared_direct_simple_insert`       | 3.62% |
|   4 | `<std::path::Path>::_starts_with`                         | 2.74% |
|   5 | `Connection::coerce_explicit_rowid_value`                 | 2.38% |

Allocator churn (`cfree` + `_int_malloc` + `__memmove_avx` +
`_int_free_merge_chunk` + `RawVec::finish_grow` + `Vec<u16>::clone`)
collectively totals ~14% but is distributed across workload paths;
several recent landings already targeted it (`d9c410bb`, `7e4a5409`,
`daf81b39`, `b96a4e38`).

## Fix landed against rank 1 (bd-5q7jn): `clear_page_data(&mut self)`

`store_prepared_direct_insert_append_hint` was doing two by-value
moves of a ~100-byte `TableAppendHint` per INSERT just to strip the
cached page-data reference:

```rust
if !retain_for_memory_autocommit
    && let Some(cached_leaf) = hint.cached_leaf.take()
{
    hint.cached_leaf = Some(cached_leaf.without_page_data());
}
```

`TableAppendHint` carries a `BtreePageHeader` (`Copy`, ~32 B) plus
the 48-byte `Option<PageData>` plus smaller fields; `take` + consume
+ `Some(..)` = **~200 B of pointless memcpy per INSERT**. At the
captured rate of 657 k wps that is ~130 MB/s of bytes moved for the
sole purpose of writing `None` into one Option field.

Added `TableAppendHint::clear_page_data(&mut self)` in
`fsqlite-btree/src/cursor.rs` and rewrote the caller in
`fsqlite-core/src/connection.rs` to use it in place:

```rust
if !retain_for_memory_autocommit
    && let Some(cached_leaf) = hint.cached_leaf.as_mut()
{
    cached_leaf.clear_page_data();
}
```

Zero struct moves. The existing consuming `without_page_data(self) -> Self`
is retained because the second caller
(`prepare_prepared_direct_insert_cached_leaf_hint`) genuinely
consumes + returns the value.

Commit: same PR as this artifact. Regression gate piggy-backs on the
standing cumulative mt_mvcc_bench sweep (see
`cumulative-verify-20260424T2033Z/`); next re-run should show
`store_prepared_direct_insert_append_hint` drop in the 1t self-time
ranking.

## Other candidates flagged for follow-up

Not fixed in this PR, but documented here so the swarm can pick
them up:

- **Rank 4 — `Path::starts_with` 2.74%**: surprising for a steady-
  state INSERT loop. Best guess: `path == ":memory:"` checks or
  `journal_path()` computation being invoked per-call instead of
  cached. Worth tracing to attribute.
- **Rank 2 — `read_cell_pointers_into` 3.78%**: btree-side
  (not pager/wal). Paging-in cost of cell-slot cache misses on
  the rightmost leaf. Candidate for widened `CachedCellSlots`
  inline capacity (already partially done in `1be6ee30`) or a
  last-cell-slot fast path.
- **Rank 3 — `execute_prepared_direct_simple_insert` 3.62%**: the
  full INSERT dispatch itself. Hard to shrink without restructuring;
  the 60 ns/row budget is already tight.
- **Allocator cluster ~14%**: further Vec-preallocation sweeps
  (especially in the VDBE side-band / ceremony-amortization path)
  would compound with the recent landings.

## Bench result context

1-thread fs_wps of 657 k in this capture (vs 407 k in the
`cumulative-verify-20260424T2033Z/` sweep) is higher because this
capture uses `--rows-per-thread=2000 --iters=15` (larger
steady-state work window = more amortization across BEGIN/COMMIT
boundaries), whereas the cumulative verify used the stricter
`--rows-per-thread=500 --iters=10`. Both numbers reflect the same
binary and HEAD; the apples-to-apples comparison against SQLite
stays 0.49× at 2000-rows and 0.56× at 500-rows, showing the 1t gap
is steady-state rather than startup-driven.

## Artifacts in this directory

- `summary.md` — this file
- `perf-top20.txt` — raw `perf report --stdio --no-children`
  top-20 listing
- `bench.stdout` / `bench.stderr` — single-run output that fed
  `perf record`
- `head.txt` — HEAD at capture
- `hostinfo.txt` — kernel version

`perf.data` (11 MB binary) omitted per repo convention; re-capture
with the command at the top of this file.
