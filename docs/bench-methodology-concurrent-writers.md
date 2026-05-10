# Benchmark methodology: concurrent writers

## TL;DR

`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs::bench_concurrent_writers`
now runs FrankenSQLite and C SQLite with the same shape: N OS threads, one
connection per thread, one shared file-backed WAL database, and disjoint rowid
ranges. Current full-matrix concurrent rows are therefore valid multi-writer
MVCC measurements.

Use `crates/fsqlite-e2e/src/bin/mt_mvcc_bench.rs` (IMPL-4a) when you want the
standalone scale harness: 1/2/4/8/16-thread reports, separate-table mode,
startup diagnostics, and pass-over-pass history gates.

## Background

FrankenSQLite's `Connection` type is `!Send + !Sync` — its internal state
includes `RefCell` and `Rc` fields that cannot cross thread boundaries.
For a benchmark to run true concurrent writers, it must construct
*one Connection per OS thread*, each bound to the same file-backed
database, and coordinate them at the MVCC/WAL layer below the Connection
API.

`bench_concurrent_writers` was originally written against the `rusqlite`
baseline, which *does* have a `Send` Connection. When the FrankenSQLite
baseline was first added, the loop iterating 1..N "writers" was left as a
sequential for-loop over a single Connection, with each "writer" performing a
transaction serially. Those older artifacts are apples-to-oranges and should
not be used for current MVCC claims.

The current implementation has been corrected: each FrankenSQLite worker opens
its own `Connection::open(path)` inside its worker thread, enables concurrent
mode, and runs `BEGIN CONCURRENT` against the same file-backed database. That
matches the current C SQLite WAL arm's one-connection-per-thread shape.

## Why this matters

Several optimization items in the current campaign (IMPL-4 flat-combining
page lock table, IMPL-14 Cicada read-ts batching, IMPL-15 Hekaton TID
gap reservation, IMPL-16 Silo epoch group commit, IMPL-24 MICA
partitioned commit log) target multi-writer contention. Older
`bench_concurrent_writers` artifacts cannot measure those accurately. Current
artifacts can, but `mt_mvcc_bench` remains the preferred focused harness when
the optimization is specifically about concurrent writer scaling.

- `IMPL-4` (flat-combining) was **refused** by the implementing agent
  after discovering that the feature was already wired behind
  `mvcc-flat-combining` and that the bench could not observe any
  difference because writers were sequential.
- Apparent "4.72× faster at 8 writers" in earlier reports was not a
  FrankenSQLite win — it was a sequential-vs-multi-threaded comparison
  that happened to favor the sequential side under low per-op cost.

## What IMPL-4a provides

`mt_mvcc_bench` spawns N OS threads, each opening its own
`Connection::open(path)` against a shared file-backed database, each
running BEGIN CONCURRENT (or BEGIN for fallback), and each committing a
fixed number of rows. It measures wall-clock throughput and compares
against a matched rusqlite WAL-mode workload.

The numbers it reports are directly comparable because both sides run
the same count of OS threads performing the same count of transactions.

## When to use which bench

| Use case | Use |
|---|---|
| Single-connection latency | `comprehensive_bench::bench_*` (all but concurrent_writers) |
| Full-matrix concurrent row | `comprehensive_bench::bench_concurrent_writers` |
| Real multi-thread MVCC throughput | `mt_mvcc_bench` (IMPL-4a) |
| Cross-process conflict | `swarm_multiprocess` / `swarm_peer_visibility` |

## Before you modify `bench_concurrent_writers`

Keep it aligned with `mt_mvcc_bench`: one connection per worker thread, shared
file-backed database for shared-table mode, disjoint rowid ranges, prepared
statements on both engines, and transaction-level retry for transient MVCC
errors. If you change its workload shape, update this document and the README
performance artifact citations in the same commit.

## Related

- Campaign memory: `session_2026_04_18_ag_aac_campaign.md` — INSIGHT #75
- Blocked-by: IMPL-4, IMPL-14, IMPL-15, IMPL-16, IMPL-24
