# Current-HEAD Concurrent Writer Boundary Refresh

Date: 2026-05-12

Commit under review: `4aa78efbd07cb2572a8de22b10e533c6e08c0806`

## Commands

Release-perf build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-current-dml-target CARGO_BUILD_JOBS=2 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Focused concurrent profile:

```bash
env FSQLITE_BENCH_PROFILE_CONCURRENT=1 \
  /data/tmp/frankensqlite-codex-current-dml-target/release-perf/comprehensive-bench \
  --quick --filter concurrent \
  --json-out tests/artifacts/perf/codex-current-head-concurrent-profile-20260512T0212/concurrent-profile.json \
  --no-html
```

Clean focused repeat:

```bash
/data/tmp/frankensqlite-codex-current-dml-target/release-perf/comprehensive-bench \
  --quick --filter concurrent \
  --json-out tests/artifacts/perf/codex-current-head-concurrent-profile-20260512T0212/concurrent-clean.json \
  --no-html
```

The benchmark binary still warns that it predates HEAD because the intervening
commits were docs/artifact commits, not Rust source changes; Cargo reported the
release-perf binary up to date.

## Clean Repeat

The clean concurrent repeat reported:

- 2 writers x 1000 rows: C `13.250728 ms`, F `14.208683 ms`, ratio `1.072x`.
- 4 writers x 1000 rows: C `19.741346 ms`, F `20.892271 ms`, ratio `1.058x`.
- 8 writers x 1000 rows: C `91.496393 ms`, F `48.676111 ms`, ratio `0.532x`.
- Concurrent-only average/geomean F/C: `0.8875316916 / 0.8451717232`.

This keeps the high-thread scaling story intact but leaves low-thread
file-backed concurrent writers slightly behind C SQLite.

## Profile Counters

The profiled run showed the same shape as the prior low-thread boundary:

- Direct INSERT stayed fully on the fast path: `direct_insert == fast`,
  `slow=0` for all 2/4/8 writer rows.
- File-backed pending page-runs remained inactive:
  `page_run_flushes=0`, `page_run_records=0`.
- The 2-writer row had `mvcc_page_lock_waits=12`,
  `mvcc_busy_retries=12`, `mvcc_stale_snapshot=12`, and
  `mvcc_page_lock_wait_ns=19135098`.
- The 4-writer row had `mvcc_page_lock_waits=78`,
  `mvcc_busy_retries=78`, `mvcc_stale_snapshot=72`, and
  `mvcc_page_lock_wait_ns=145461028`.
- The 8-writer row had `mvcc_page_lock_waits=423`,
  `mvcc_busy_retries=423`, `mvcc_stale_snapshot=328`, and
  `mvcc_page_lock_wait_ns=900717737`, but still beat C SQLite cleanly.

The `candidate_free_fast_paths=0` counter is not a missed cheap path here. The
source read confirmed that the candidate-free prepare path is gated by
hydrated SSI read/write witnesses and active/committed candidate sets; this
workload still records page-level witnesses for overlapping B-tree pages even
when row key ranges do not overlap.

## Outcome

No source patch was attempted. The remaining concurrent gap is still the
transaction-level stale-snapshot/page-lock replay and MVCC publication shape,
not parser dispatch, row serialization, commit-prep key-vector allocation, or a
small witness bookkeeping issue. Adjacent standalone attempts are already
fenced in the negative-results ledger: active-holder preemption, wait-slice
tuning, file-backed page-run admission, preserialized-record widening,
candidate/witness summary reuse, and exact read-witness dedupe.
