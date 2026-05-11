# Concurrent Profile JSON Probe

- Date: 2026-05-11
- Command:
  `FSQLITE_BENCH_PROFILE_CONCURRENT=1 /tmp/frankensqlite-codex-concurrent-profile-json/release-perf/comprehensive-bench --quick --filter concurrent --json-out tests/artifacts/perf/codex-concurrent-profile-json-20260511T1825Z/concurrent.json --no-html`
- Purpose: prove the comprehensive benchmark JSON now carries the same
  concurrent hot-path counters previously emitted only in the human
  `concurrent_profile` line.
- Build provenance: release-perf benchmark binary built by
  `rch exec -- env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-concurrent-profile-json CARGO_BUILD_JOBS=4 cargo ...`

## JSON Evidence

Each concurrent-writer row contains `fsqlite_concurrent_profile`.

| Row | ratio F/C | mvcc_busy_retries | mvcc_stale_snapshot | mvcc_page_lock_waits | mvcc_page_lock_wait_ns | mvcc_tier0 | mvcc_tier1 | wal_frames |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 1.2494 | 12 | 12 | 12 | 16788870 | 8204 | 188 | 248 |
| 4 writers x 1000 rows | 1.1605 | 87 | 72 | 87 | 155575231 | 31633 | 616 | 507 |
| 8 writers x 1000 rows | 0.6575 | 473 | 322 | 473 | 908908891 | 87567 | 1650 | 1101 |

Validation query:

```sh
jq '.sections[] | select(.section_id=="concurrent-writers-c-sqlite-wal-vs-frankensqlite-mvcc") | .rows[] | {scenario_id, ratio_fsqlite_over_csqlite, profile_present: has("fsqlite_concurrent_profile"), profile: .fsqlite_concurrent_profile | {total_rows, fsqlite_median_ms, mvcc_busy_retries: .counters.mvcc_busy_retries, mvcc_stale_snapshot: .counters.mvcc_stale_snapshot, mvcc_page_lock_waits: .counters.mvcc_page_lock_waits, mvcc_page_lock_wait_ns: .counters.mvcc_page_lock_wait_ns, mvcc_tier0: .counters.mvcc_tier0, mvcc_tier1: .counters.mvcc_tier1, mvcc_tier2: .counters.mvcc_tier2, wal_frames: .counters.wal_frames}}' tests/artifacts/perf/codex-concurrent-profile-json-20260511T1825Z/concurrent.json
```
