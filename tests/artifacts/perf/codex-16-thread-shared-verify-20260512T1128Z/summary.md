# 16-thread shared-table retry verification

Date: 2026-05-12
Commit under test before patch: `3daf8c0c35bf130e39666ed9ade2135a56a75f37`
Workload: `mt-mvcc-bench --rows-per-thread=1000 --threads=16 --iters=3`

## Finding

The pre-patch run could still fail under the 16-thread shared-table shape. One
observed failure exhausted the fixed 32-retry budget and then continued issuing
INSERTs inside the same transaction, producing repeated `BUSY_SNAPSHOT` errors
on page 2 and finally failing COMMIT on pages `76,77,78,79,80,81,82,83,84`.
See `before-fix-failure-excerpt.txt` for the compact failure excerpt.

That exposed two harness bugs:

- The FSQLite transaction retry loop allowed only about 32ms of app-level
  `BUSY_SNAPSHOT` retries despite configuring `PRAGMA busy_timeout=5000`.
- After retry exhaustion on INSERT, the harness treated transient
  `BUSY_SNAPSHOT` as a row failure and continued the transaction. SQLite
  semantics require rolling back and restarting the whole transaction, or
  failing the worker if the retry budget is exhausted.

## Patch

`crates/fsqlite-e2e/src/bin/mt_mvcc_bench.rs` now uses a timeout-scaled
transaction retry budget with deterministic jittered backoff. Transient
BEGIN/INSERT/COMMIT failures always roll back and retry the whole transaction
while budget remains; if the budget is exhausted, the worker returns a hard
error instead of recording bogus failed rows.

The pass-over-pass gate now ignores historical reports with a different
`workload_shape` or `rows_per_thread`, preventing false failures when the same
history path is reused for probes with different row counts.

## Proof

Targeted unit tests:

`cargo test -p fsqlite-e2e --bin mt-mvcc-bench -- --nocapture`

Result: 9 passed.

Post-patch verification command:

`mt-mvcc-bench --rows-per-thread=1000 --threads=16 --iters=3 --history-json=tests/artifacts/perf/codex-16-thread-shared-verify-20260512T1128Z/history-after-fix-16t-1000r-iters3.json --json-output=tests/artifacts/perf/codex-16-thread-shared-verify-20260512T1128Z/after-fix-16t-1000r-iters3.json --summary-md=tests/artifacts/perf/codex-16-thread-shared-verify-20260512T1128Z/summary-after-fix.md`

Result: exit 0, `0` FSQLite failed rows, `0` C SQLite failed rows.

Key row:

| Threads | F writes/sec | C writes/sec | Throughput F/C | F p50 ms | C p50 ms | Failed rows |
|---:|---:|---:|---:|---:|---:|---:|
| 16 | 226357 | 25278 | 8.95x | 70.68 | 632.96 | 0 |
