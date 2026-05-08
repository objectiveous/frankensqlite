# Current Concurrent Frontier Repeat

Date: 2026-05-08
Agent: WindyIbis
Source commit: `a13ebebdc81aa7a47a59987d991d3f5c4a8fce90`

## Purpose

After the direct UPDATE scratch-borrow candidate was rejected, this pass
rechecked the remaining low-thread concurrent-writer frontier before touching
source again. The goal was to distinguish a stable source target from benchmark
variance and already-ledgered micro-optimizations.

## Coordination

- Reserved artifact path:
  `tests/artifacts/perf/windyibis-current-frontier-repeat-20260508T091105Z/**`
  as reservation `13328`.
- A start-message attempt to Agent Mail timed out under database contention.
- `crates/fsqlite-core/src/connection.rs` became dirty while this pass was
  producing artifacts. A reservation check showed it was held by `BoldLion`
  until `2026-05-08T10:46:16Z`, so this pass did not edit, revert, stage, or
  benchmark-source-gate that file.
- To avoid dirty-git benchmark metadata, authoritative focused reports were
  rerun from detached clean worktree
  `/data/tmp/frankensqlite-windyibis-clean-a13` at the source commit above.

## Build

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-current-frontier-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-current-frontier-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin mt-mvcc-bench --profile release-perf
```

The retained binaries were:

```text
/data/tmp/frankensqlite-windyibis-current-frontier-target/release-perf/comprehensive-bench
/data/tmp/frankensqlite-windyibis-current-frontier-target/release-perf/mt-mvcc-bench
```

## Authoritative Clean Concurrent Repeat

All three `clean-concurrent-repeat*.json` reports recorded
`git_dirty=false`, `git_commit_sha=a13ebebd...`, and
`benchmark_binary_older_than_git_head=false`.

| Report | Scenario | Ratio F/C | FSQLite ms | C SQLite ms | F CV % | C CV % |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `clean-concurrent-repeat1` | 2 writers x 1000 rows | `1.106709` | `13.637991` | `12.323019` | `5.54` | `19.44` |
| `clean-concurrent-repeat1` | 4 writers x 1000 rows | `1.007860` | `19.776347` | `19.622118` | `13.45` | `9.42` |
| `clean-concurrent-repeat1` | 8 writers x 1000 rows | `0.371607` | `33.656682` | `90.570691` | `8.41` | `3.65` |
| `clean-concurrent-repeat2` | 2 writers x 1000 rows | `1.070327` | `13.789245` | `12.883208` | `5.03` | `3.21` |
| `clean-concurrent-repeat2` | 4 writers x 1000 rows | `0.967261` | `19.655340` | `20.320607` | `7.48` | `3.91` |
| `clean-concurrent-repeat2` | 8 writers x 1000 rows | `0.411239` | `37.576153` | `91.372963` | `12.86` | `15.23` |
| `clean-concurrent-repeat3` | 2 writers x 1000 rows | `1.154545` | `14.445524` | `12.511872` | `20.04` | `4.85` |
| `clean-concurrent-repeat3` | 4 writers x 1000 rows | `1.020825` | `19.576352` | `19.176994` | `14.01` | `21.86` |
| `clean-concurrent-repeat3` | 8 writers x 1000 rows | `0.411789` | `37.360889` | `90.728336` | `80.59` | `0.68` |

The 2-writer row stayed slower than C SQLite in the clean repeat
(`1.070x` to `1.155x`). The 4-writer row stayed parity, and the 8-writer row
remained substantially faster despite high variance.

## Standalone 2-Thread Profile

`clean-mt-mvcc-2t-current.json`:

```text
threads | fsqlite_wps | sqlite_wps | throughput_ratio | fsqlite_ms_p50 | sqlite_ms_p50 | time_ratio
      2 |      490985 |     885208 |             0.55x |           4.07 |          2.26 |       1.80x
```

`clean-mt-mvcc-2t-perf.json` under `perf record`:

```text
threads | fsqlite_wps | sqlite_wps | throughput_ratio | fsqlite_ms_p50 | sqlite_ms_p50 | time_ratio
      2 |      442850 |     767105 |             0.58x |           4.52 |          2.61 |       1.73x
```

Top useful self-time entries from `clean-perf-mt-mvcc-2t-report.txt`:

| Symbol | Self % | Interpretation |
| --- | ---: | --- |
| `WalChecksumTransform::for_wal_frame` | `3.25` | Already covered by rejected WAL checksum header-transform attempt. |
| `PrecomputedSerialTypeKind::serial_byte_and_payload_len` | `2.92` | Adjacent record-header specialization attempts are ledgered. |
| `Connection::execute_prepared_direct_simple_insert` | `2.67` | Direct INSERT lane is saturated by recent rejected setup/row-builder probes. |
| `read_cell_pointers_into` | `2.44` | B-tree/right-edge path; many rightmost/append-hint probes are ledgered. |
| `BtCursor<SharedTxnPageIo>::load_page` | `1.98` | Shared page-I/O path; standalone bypass/page-run probes have failed gates. |
| `ConcurrentHandle::ensure_page_state` | `1.32` | Adjacent prepared commit page-set `SmallVec` probe rejected. |
| `table_try_append_cached_rightmost_leaf_hint` | `0.98` | Rightmost append-hint family is heavily ledgered. |

## Superseded Dirty-Cwd Reports

The files `concurrent-repeat*.json`, `mt-mvcc-2t-current.*`,
`mt-mvcc-2t-perf.*`, and `perf-mt-mvcc-2t-*` were captured before the clean
detached worktree rerun. They are kept for audit only. The authoritative
clean-git reports are the files prefixed with `clean-`.

## Decision

No source candidate was attempted or kept. The clean matrix confirms a real but
small low-thread concurrent gap, while the profile points at surfaces already
rejected as standalone micro-optimizations: WAL checksum header work,
record-header specialization, rightmost append hints, and prepared concurrent
page-set reshaping.

The next keepable concurrency attempt should be a broader design, not a
single-symbol tweak: either reduce WAL payload/prepared-frame pipeline cost or
remove a larger unit of repeated direct-INSERT/SharedTxnPageIo ceremony. It
must improve the clean 2-writer row and the full quick matrix in the same A/B
window without sacrificing the 4/8-writer rows.
