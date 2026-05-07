# Lazy waiter-shard allocation candidate

## Scope

- Target workload: small `:memory:` UPDATE/DELETE and fresh-open fixed costs where
  `perf-update-delete` still showed allocation/open cost in
  `SharedMvccState::new` / `InProcessPageLockTable`.
- Touched source during rejected candidate:
  `crates/fsqlite-mvcc/src/core_types.rs`.
- Candidate shape: replace eagerly allocated page-lock waiter shards with
  `OnceLock`-backed shards, allocating a waiter shard only when a thread
  actually registers as parked on a page lock.

## Correctness proof

- `cargo fmt -p fsqlite-mvcc --check` passed.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazy-waiter-target cargo test -p fsqlite-mvcc in_process_lock_table -- --nocapture`
  printed `7 passed; 0 failed; 1 ignored`.
- The RCH wrapper later hung while retrieving target artifacts after the green
  test result, so it was terminated locally.

## Focused same-window probe

Compared against the current-HEAD baseline binary at
`/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target`.

| Row | Baseline | Candidate | Direction |
| --- | ---: | ---: | --- |
| 100-row UPDATE standard | 1538 ns/updated row | 1520 ns/updated row | improved 1.2% |
| 100-row DELETE standard | 2417 ns/deleted row | 2453 ns/deleted row | regressed 1.5% |

## Full quick matrix

Baseline report:
`tests/artifacts/perf/lock-table-shards-64-purpleotter-20260507T064123Z/report-full.json`.

Candidate report:
`tests/artifacts/perf/lazy-waiter-shards-crimsongorge-20260507T0715Z/report-full.json`.

| Metric | Baseline | Candidate | Direction |
| --- | ---: | ---: | --- |
| Primary weighted score | 0.370574 | 0.381847 | regressed |
| Average ratio | 0.512508 | 0.517770 | regressed |
| Geomean ratio | 0.277310 | 0.287617 | regressed |
| C SQLite faster rows | 14 | 17 | regressed |
| FrankenSQLite faster rows | 73 | 71 | regressed |
| Write-single geomean | 1.192892 | 1.209958 | regressed |
| Write-bulk geomean | 0.951178 | 0.945722 | improved |

## Disposition

Rejected and reverted before commit. The idea is mechanically plausible, but
the matrix says it hurts the current system more than it helps. Do not retry
standalone lazy waiter-shard allocation unless a future profile shows waiter
shard construction as retained self-time after a broader lock-table/open-state
redesign.
