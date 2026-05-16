# Current Concurrent Profile Boundary

## Scope

- Repo: `/data/projects/frankensqlite`
- Benchmarked source: `main @ b871658b255ac9aeb9003f4383b00ed5a5518dee`
- Run date: 2026-05-16
- Raw output: `run.log`
- Command:

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_CONCURRENT=1 CARGO_TARGET_DIR=/tmp/frankensqlite-current-concurrent-profile \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter concurrent \
  --json-out tests/artifacts/perf/codex-current-concurrent-profile-b871658b-20260516T2320Z/concurrent.json \
  --no-html
```

RCH reported that `concurrent.json` was written remotely, but the JSON artifact
was not retrieved locally; `run.log` is the preserved local evidence.

## Matrix

| Scenario | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| 2 writers x 1000 rows | 3.59 ms | 6.13 ms | 1.71x slower |
| 4 writers x 1000 rows | 11.59 ms | 15.71 ms | 1.36x slower |
| 8 writers x 1000 rows | 83.52 ms | 52.85 ms | 1.58x faster |

Summary line: 3 scenarios, 1 FrankenSQLite faster, 0 comparable, 2 C SQLite
faster, average F/C ratio 1.23x.

## Representative Profile Counters

2 writers:

- `direct_insert=24012 fast=24012 slow=0`
- `concurrent_plan_attempts=36 successes=36 errors=0 busy_snapshot_errors=0`
- `concurrent_plan_uncontended_fast_paths=24`
- `concurrent_plan_candidate_free_fast_paths=0`
- `concurrent_plan_full_validations=12`
- `mvcc_page_lock_waits=12`
- `mvcc_page_lock_wait_ns=18404225`
- `mvcc_busy_retries=12`
- `mvcc_stale_snapshot=12`

4 writers:

- `direct_insert=55130 fast=55130 slow=0`
- `concurrent_plan_attempts=67 successes=60 errors=7 busy_snapshot_errors=7`
- `concurrent_plan_uncontended_fast_paths=24`
- `concurrent_plan_candidate_free_fast_paths=0`
- `concurrent_plan_full_validations=36`
- `mvcc_page_lock_waits=84`
- `mvcc_page_lock_wait_ns=156020480`
- `mvcc_busy_retries=84`
- `mvcc_stale_snapshot=72`

8 writers:

- `direct_insert=129524 fast=129524 slow=0`
- `concurrent_plan_attempts=139 successes=108 errors=31 busy_snapshot_errors=31`
- `concurrent_plan_uncontended_fast_paths=32`
- `concurrent_plan_candidate_free_fast_paths=0`
- `concurrent_plan_full_validations=76`
- `mvcc_page_lock_waits=441`
- `mvcc_page_lock_wait_ns=994543679`
- `mvcc_busy_retries=441`
- `mvcc_stale_snapshot=320`

## Interpretation

The current same-table concurrent workload is not on the slow VDBE path:
`direct_insert` is all fast-path, `page_run_flushes=0`, and `mvcc_tier2=0`.
The remaining low-thread red rows are dominated by MVCC page-lock holder churn
and stale snapshots, while 8 writers still beats C SQLite once C SQLite's
single-writer WAL lock becomes the bottleneck.

This artifact does not justify another standalone retry-loop, wait-slice,
candidate-free fast-path, release-set wakeup, or file-backed page-run admission
patch. The next credible concurrent lever is broader: batch file-backed page
construction and MVCC publication together while preserving first-committer-wins
and SSI behavior, then prove the focused concurrent row and full quick matrix.
