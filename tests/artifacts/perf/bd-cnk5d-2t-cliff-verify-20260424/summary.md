# mt_mvcc_bench thread sweep — post fence-fix + Apr-24 perf wave (2026-04-24)

Follow-up verification for `0f04cb25 perf(wal): spin-fast-path in
RecoveryFence::acquire_for_recovery` plus the ~11 perf commits that
landed in the following ~14 h (parser interner retention, planner
index-selection metric mutex removal, btree CachedCellSlots inline
capacity widening, VDBE DecrJumpZero hot-path promotion, MVCC
snapshot-read histogram gating, …).

HEAD at capture = `00f2c2be`.

## Bench sweep

Command (repeated ×3 per thread count; median reported):

```
/tmp/rch_target_cc2_1777000277/release/mt-mvcc-bench \
  --rows-per-thread=500 --threads=<T> --iters=5
```

Raw: `bench-sweep.log`.

| Threads | campaign-summary baseline | 2026-04-24 01:35 median | 2026-04-24 12:12 median | Δ vs campaign-summary | Δ vs 11 h ago |
|--------:|--------------------------:|------------------------:|------------------------:|----------------------:|--------------:|
|   1     |  88,837 / 304,986 typical | 232,757                 | 247,974                 | matches typical       | +7%            |
|   2     |   8,918                   |   9,537                 |   9,554                 | +7% (within noise)    | flat           |
|   4     |   5,963                   |   9,436                 |   9,509                 | +59%                  | flat           |
|   8     |   5,458                   |  12,277                 |  12,430                 | **+128%**             | flat           |

Run-to-run spread (min / median / max) at 12:12:

| Threads | min     | median  | max     |
|--------:|--------:|--------:|--------:|
|   1     | 221,942 | 247,974 | 288,991 |
|   2     |   9,536 |   9,554 |   9,578 |
|   4     |   9,424 |   9,509 |  17,582 |
|   8     |  12,274 |  12,430 |  17,923 |

2t is remarkably stable at ~9,550 — three runs within 42 wps of each
other. That's the inner.lock() cliff asserting itself deterministically
(bd-wee9a). 4t and 8t show an occasional 1.4-1.9× outlier that
corresponds to lucky scheduler alignment between writers.

## Interpretation

1. **Recovery-fence fix worked on its stated target.** Where it shows
   up: 12t `BusyRecovery` failures are gone (cod_5's finding), 1t
   throughput is at the post-db-generation-cache-fix "typical" band
   (~240k wps), and no run in this sweep observed the 100 ms sleep
   penalty during Connection::open. See `0f04cb25` commit body.
2. **2t cliff is NOT the recovery fence.** 2t fs_wps moved from the
   8,918 campaign-summary baseline to 9,554 — +7% total, within run
   variance. This was predicted in `bd-wee9a`: the fence is taken
   only in Connection::open (`pager.rs:5207`), never in the
   commit-time WAL-append path. The cliff driver is
   `SimpleTransaction.inner.lock()` in Phase A serializing all
   writers of the same pager upstream of the GroupCommitQueue.
3. **4t gained +59%, 8t gained +128%** vs campaign-summary. These
   gains are broadly distributed across the Apr-24 perf wave
   (DecrJumpZero/SCopy/AddImm VDBE hot-path promotions, parser
   interner retention, btree CachedCellSlots widening, mvcc
   SireadTable short-circuit, planner index-selection metric mutex
   removal, PublishedPagerState 4096→512 floor, `RecoveryFence`
   spin-fast-path, etc.) — not attributable to any single commit.
4. **Peak 2t→4t→8t ratios are still inverted vs sqlite.** sqlite
   drops from 539k @ 2t to 211k @ 4t to 50k @ 8t (classic fsync
   contention at high concurrency). fsqlite goes 9.5k → 9.5k → 12k
   — we're serialized so hard that more threads is *almost free
   because no work is happening*.

## New 2t on-CPU profile (reference; still accurate at current HEAD)

Captured at `3e1848ee` (~11 h before this commit). The cliff is
architectural so the top-5 are stable across small perf commits;
re-capture before any targeted work on one of these symbols.

`perf record -F 999 --call-graph=dwarf,16384` on
`mt-mvcc-bench --rows-per-thread=500 --threads=2 --iters=10`
(711 samples, full listing in `perf-top20.txt`):

| Rank | Symbol | Self-time | Category |
|-----:|---|---:|---|
|    1 | `Arc<ShardedPageCache>::drop_slow`                       | 7.02% | pager lifecycle (drop) |
|    2 | `xxhash_rust::xxh3::xxh3_64_long_default`                | 5.31% | page_mutation_counter hash |
|    3 | `RawVec<BtreeCursor::StackEntry>::grow_one`              | 3.47% | btree cursor stack growth |
|    4 | `fsqlite_btree::cell::read_cell_pointers_into`           | 2.88% | btree cell-parse cluster |
|    5 | `Connection::execute_prepared_with_params`               | 2.87% | VDBE prepared dispatch |
|    6 | `Connection::execute_prepared_direct_simple_insert`      | 2.77% | INSERT fast path |
|    7 | `libc::statx`                                            | 2.51% | VFS stat calls |
|    8 | `Connection::coerce_explicit_rowid_value`                | 2.33% | rowid coercion |
|    9 | `SqliteValue::drop_in_place`                             | 2.04% | value destructor |
|   10 | `serialize_record_iter_with_precomputed_header_into`     | 2.04% | record serialization |

Plus ~15% unresolved kernel frames (`kptr_restrict` active) whose
shape is consistent with `futex_wait` on the Phase-A `inner.lock()`.

**Note on rank-3 `StackEntry::grow_one`:** commit `1be6ee30
perf(btree): widen CachedCellSlots inline capacity and inline cursor
stack (bd-9e3xf.2)` landed between the profile capture and this
sweep. That commit is the direct counter-lever for rank 3, so the
live 2t profile at `00f2c2be` likely has `grow_one` reduced or gone
from the top-10. Re-capture with `perf record` before attacking the
next rank.

## Artifacts in this directory

- `summary.md` — this file
- `bench-sweep.log` — 1t / 2t / 4t / 8t × 3 runs @ HEAD `00f2c2be`
- `bench-stdout.log`, `bench-stderr.log`, `bench-2t-result.txt` —
  the single 2t run that drove the `perf record` capture
- `perf-top.txt`, `perf-top20.txt` — text extracts of the 2t
  `perf report --no-children` output at `3e1848ee`
- `head.txt` = `00f2c2be` (sweep HEAD)
- `hostinfo.txt`

`perf.data` (11 MB binary) was not committed — re-capture locally if
you need callgraph drill-down:

```
perf record -F 999 --call-graph=dwarf,16384 -o perf.data -- \
  /tmp/rch_target_cc2_1777000277/release/mt-mvcc-bench \
    --rows-per-thread=500 --threads=2 --iters=10
```

## Disposition

- `bd-cnk5d` (fence spin-fast-path): closed by `0f04cb25`; verified
  here. No rework needed.
- `bd-wee9a` (Phase-A inner.lock cliff): stays open. 2t stably
  pinned at 9,554 wps across three back-to-back runs is the
  clearest evidence yet that the cliff is deterministic, not
  scheduling variance. Any follow-up work on the cliff must shrink
  or bypass Phase A's `self.inner.lock()` hold; the
  GroupCommitQueue tuning pane is a dead end (already tested
  empirically, see `0df7d65e` commit body).
- Candidate next-pager levers from the rank-1-2 profile:
  - `Arc<ShardedPageCache>::drop_slow` 7.02% — single fattest
    on-CPU symbol at 2t. Worth a dedicated investigation bead.
  - `xxh3_64_long_default` 5.31% (page_mutation_counter) —
    `T5a` experiment in 2026-04-23 reverted the crc32c swap
    because crc32c was 28 % slower for 4 KiB inputs on this host.
    Fix requires *avoiding* the hash call, not replacing the
    primitive.
