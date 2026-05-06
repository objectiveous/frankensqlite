# Rejected Direct UPDATE/DELETE Reusable Page-I/O Shell

Agent: ProudAnchor
Date: 2026-05-06

## Target

Current full quick matrix after `7d6117e1` showed the worst remaining row in
`UPDATE/DELETEThroughput`:

- `100 rows / delete 5 rows`: C SQLite `0.092583 ms`, FrankenSQLite
  `0.425427 ms`, ratio `4.5950876511`.

The narrow isolated profiler confirmed the small DELETE gap:

- Baseline `perf-update-delete 100 20000 delete compare isolated`:
  FrankenSQLite `1580 ns/delete`, C SQLite `293 ns/delete`, ratio `5.40x`.

## Profile

Delayed `perf record` was used to skip most populate work and sample the DELETE
loop:

```bash
perf record --delay 80 -F 999 -o delete100-fsqlite-isolated-delay.perf.data -- \
  /data/tmp/frankensqlite-proudanchor-updatedelete/release-perf/perf-update-delete \
  100 20000 delete fsqlite isolated
perf report --stdio --no-children -i delete100-fsqlite-isolated-delay.perf.data --sort overhead,symbol
```

Top user-space symbols included:

- `TransactionKind::get_page` - `14.59%`
- `__memmove_avx_unaligned_erms` - `13.48%`
- `BtCursor<SharedTxnPageIo>::delete` - `11.22%`
- `_int_malloc` - `8.42%`
- `TransactionKind::write_page_data` - `5.62%`
- `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` - `4.11%`

See `delete100-fsqlite-isolated-delay-perf-report.txt`.

## Candidate

Reused a drained `SharedTxnPageIo` shell across direct UPDATE/DELETE executions
inside explicit transactions:

- Added refill/drain reuse methods on `SharedTxnPageIo`.
- Added a cached `SharedTxnPageIo` shell on `Connection`.
- Routed direct-simple UPDATE/DELETE concurrent paths through the cached shell.

The intent was to remove per-row Rc/RefCell wrapper allocation while preserving
the active concurrent transaction and page-level MVCC semantics.

## Measurement

Same target dir, old binary before rebuild as baseline, then rebuilt candidate:

```bash
/data/tmp/frankensqlite-proudanchor-updatedelete/release-perf/perf-update-delete \
  100 20000 delete compare isolated
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-updatedelete \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete \
  100 20000 delete compare isolated
```

Result:

| Run | FrankenSQLite delete | C SQLite delete | Ratio |
| --- | ---: | ---: | ---: |
| Baseline | `1580 ns/delete` | `293 ns/delete` | `5.40x` |
| Candidate | `1613 ns/delete` | `334 ns/delete` | `4.83x` |

## Decision

Rejected and source reverted. The C/FrankenSQLite ratio improved only because
the C SQLite denominator slowed down; the absolute FrankenSQLite target row
regressed by about `2.1%`.

Peer review caveat: any future version of this idea must preserve the old
`SharedTxnPageIo::into_inner()` stray-reference guard before draining or
stashing a reusable shell.

Do not retry reusable `SharedTxnPageIo` shell caching for direct UPDATE/DELETE
as a standalone optimization. Reconsider only if an allocation profile proves
the wrapper allocation dominates and a same-window A/B improves absolute
FrankenSQLite delete/update medians.
