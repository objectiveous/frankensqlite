# INSERT 100-Row Fixed-Cost Profile Refresh

Date: 2026-05-17

Source: `6b4181415c1e1a38c013b895cdca5f8ace522aaa` plus the current dirty
profiling/negative-ledger patch.

RCH had no admissible workers (`critical_pressure=6`) and failed open to local
execution.

## Commands

The first attempt used an unsupported `--rows` flag and failed after compiling
the benchmark binary:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --rows 100 --json-out tests/artifacts/perf/codex-insert-100-fixed-profile-20260517Tnext/insert-100.json --no-html
```

The supported rerun used the quick INSERT filter:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-insert-100-fixed-profile-20260517Tnext/insert-quick.json --no-html
```

## Matrix Result

Focused INSERT quick result:

- Total scenarios: `25`
- FrankenSQLite faster / comparable / C SQLite faster: `18 / 2 / 5`
- Weighted score: `0.8395106202061501`
- P99 ratio: `1.174382736306`

Rows slower than C SQLite by more than 5 percent:

| Scenario | C SQLite | FrankenSQLite | F/C | F CV |
|---|---:|---:|---:|---:|
| small_3col 100 rows / batched 100/txn | 76.183 us | 89.468 us | 1.174x | 2.41% |
| small_3col 100 rows / single transaction section | 76.834 us | 88.746 us | 1.155x | 7.81% |
| large_10col 100 rows / single transaction section | 147.466 us | 167.032 us | 1.133x | 4.00% |
| medium_6col 100 rows / single transaction section | 102.482 us | 112.079 us | 1.094x | 6.01% |
| tiny_1col 100 rows / single transaction section | 68.217 us | 74.149 us | 1.087x | 9.32% |

All 1000-row and most 10000-row INSERT rows were faster than C SQLite in this
same run. The red surface is therefore the fixed 100-row tail, not bulk INSERT.

## Hotspot Table

The profile counters add nested `Instant` overhead and should be used for rank,
not wall-clock accounting.

| Rank | Location | Signal | Representative value | Interpretation |
|---:|---|---:|---:|---|
| 1 | Direct small-record construction | `row_build_ns`, `preserialize_ns` | small_3col 100: `68.006 us` / `62.126 us` | Dominant profiled region for 100-row small inserts, but previous standalone serializer/scratch/affinity tweaks are fenced. |
| 2 | Per-statement background gate | `bg_checks`, `bg_ns` | small_3col 100: `104` / `2.842 us` | One cheap atomic per API operation; not enough as a standalone lever. |
| 3 | Schema/maintenance guards | `schema_validation_ns`, `change_tracking_ns`, `memdb_apply_ns` | small_3col 100: `3.245 us`, `2.305 us`, `2.486 us` | Fixed costs are visible only in tiny 100-row slices; prior cache-skip families failed full gates. |
| 4 | Commit and page-run flush | `commit_us`, `direct_flush_ns` | small_3col 100: `10.6 us`, `3.688 us` | Already amortizes strongly; larger INSERT rows are faster than C SQLite. |
| 5 | Large-row construction | `large_10col` 10K profile | `row_build_ns=19.821 ms`, `direct_flush_ns=2.509 ms` | Still the broad fused row/body/page construction target, but this run has only `1.02x` large-row record-size ratio. |

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Decision |
|---|---:|---:|---:|---:|---|
| Another standalone direct-record serializer tweak | 2 | 1 | 3 | 0.67 | Reject. Previous concat/scratch/layout/affinity/page-run micro-patches failed matrix gates. |
| Background-status batching or bypass | 1 | 2 | 3 | 0.67 | Reject. The gate is one atomic and only a few microseconds over 100 statements. |
| Exact benchmark setup/PRAGMA bypass | 1 | 1 | 2 | 0.50 | Reject. Already tried and rejected in the ledger. |
| Fused row/body/page construction for large records | 4 | 2 | 5 | 1.60 | Research only from this artifact; large 10K row is not a stable current red frontier. |
| Transaction-local DML mutation operator | 5 | 3 | 5 | 3.00 | Next code target remains DML, not INSERT, because it attacks the current top red rows and matches the retained-leaf ceremony profile. |

## Decision

No INSERT source patch was attempted from this refresh. The focused INSERT
section is currently healthy overall: `18` faster rows, only `5` C-faster rows,
and no stable broad INSERT bottleneck. The next performance code should stay on
the DML mutation-operator path unless a full quick refresh makes INSERT the
dominant red category again.
