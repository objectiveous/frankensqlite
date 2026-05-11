# Current Concurrent Writer Screen

- Date: 2026-05-11
- Command: `env FSQLITE_BENCH_PROFILE_CONCURRENT=1 /data/tmp/frankensqlite-current-head-insert-refresh/release-perf/comprehensive-bench --quick --filter concurrent --json-out tests/artifacts/perf/codex-current-concurrent-screen-20260511T123119Z/concurrent.json --no-html`
- Source: `3872676fccbf5d7a9beb1e82741d33f61ba50788`
- Note: the binary predates the artifact-only `3872676f` commit, but `fdba4a6f..3872676f` contains no non-artifact source changes.

## Result

The focused concurrent slice has 3 measured rows:

- Faster: 1
- Comparable: 0
- C SQLite faster: 2
- Average ratio: `0.9548678613134429`
- Geomean ratio: `0.8866013184214386`
- Median ratio: `1.181597961301353`
- P90/P99 ratio: `1.1854687435156468`

Rows:

| Row | C SQLite median | FrankenSQLite median | Ratio |
| --- | ---: | ---: | ---: |
| 2 writers x 1000 rows | 12.100268 ms | 14.297652 ms | 1.181598x |
| 4 writers x 1000 rows | 19.253077 ms | 22.823921 ms | 1.185469x |
| 8 writers x 1000 rows | 91.298808 ms | 45.424524 ms | 0.497537x |

## Profile Notes

The run stays on the prepared direct INSERT fast path:

- 2 writers: `direct_insert=24012`, `fast=24012`, `slow=0`, `page_run_flushes=0`
- 4 writers: `direct_insert=58143`, `fast=58143`, `slow=0`, `page_run_flushes=0`
- 8 writers: `direct_insert=126699`, `fast=126699`, `slow=0`, `page_run_flushes=0`

The low-thread gap is still dominated by transaction-level concurrent-writer
retry cost rather than parser or generic INSERT dispatch:

- 2 writers: `mvcc_page_lock_waits=12`, `mvcc_busy_retries=12`, `mvcc_stale_snapshot=12`, `mvcc_page_lock_wait_ns=18088806`
- 4 writers: `mvcc_page_lock_waits=80`, `mvcc_busy_retries=80`, `mvcc_stale_snapshot=69`, `mvcc_page_lock_wait_ns=140776414`
- 8 writers: `mvcc_page_lock_waits=432`, `mvcc_busy_retries=432`, `mvcc_stale_snapshot=316`, `mvcc_page_lock_wait_ns=811739356`

## Decision

No source patch from this screen. The current evidence is the same family as
the 2026-05-10 low-thread concurrent boundary: retry reshaping, wait-slice
tuning, file-backed page-run admission, lazy MemDB mirroring, WAL checksum
precompute, and small commit-page-set representation changes are already
measured negative as standalone levers. The next credible concurrent change is
a broader representation change that batches file-backed page construction with
MVCC publication while preserving first-committer-wins and proving the 2/4 rows
without regressing the 8-writer row or the full quick score.
