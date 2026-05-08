# MemDB Already-Dirty Abandon Skip - Rejected

Date: 2026-05-08

Agent: CalmThrush

Shared checkout head:
`a196a152e0c786aace663811330a1f28d787d819`
(`docs(perf): ledger rejected microprobes`)

Candidate scratch worktree:
`/data/tmp/frankensqlite-memdb-abandon-calmthrush-20260508T0200Z`

`crates/fsqlite-core/src/connection.rs` and the negative-results ledger were
reserved by CrimsonGorge, so this probe was run only in the scratch worktree.
The shared source file was not edited.

## Candidate

In `finish_prepared_direct_simple_insert_after_storage`, the direct INSERT path
calls `abandon_exact_memdb_row_mirror()` when exact MemDatabase delta tracking
is disabled. After the first private-memory preserialized INSERT row, the mirror
is already marked dirty, so later rows re-enter a helper that only confirms the
same dirty state.

The candidate skipped the helper when these three flags already represented the
dirty mirror state:

```rust
!memdb_rows_loaded
&& !memdb_storage_count_shortcuts_safe
&& memdb_requires_active_txn_reload
```

## Correctness Proof

The first test attempt used a missing `TMPDIR` and failed before exercising the
candidate. After creating the temp directory, the focused direct INSERT tests
passed:

```text
mkdir -p /data/tmp/frankensqlite-calmthrush-tmp
env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-memdb-abandon-test-target \
  CARGO_BUILD_JOBS=8 \
  cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture
```

Result: `28 passed`.

`cargo fmt -p fsqlite-core --check` also passed.

## Benchmark Gate

Candidate build:

```text
env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-memdb-abandon-bench-target \
  CARGO_BUILD_JOBS=8 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Focused INSERT A/B:

```text
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-head53367-insert-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html \
  --json-out baseline-insert.json

FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-memdb-abandon-bench-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html \
  --json-out candidate-insert.json
```

Primary summary:

| Metric | Baseline | Candidate | Verdict |
| --- | ---: | ---: | --- |
| Insert weighted score | `0.7313009035` | `0.7863189504` | worse |
| Average ratio | `0.8263544353` | `0.8228362706` | slight better |
| Geomean ratio | `0.7950067411` | `0.7968219330` | worse |
| P90 ratio | `1.1386710196` | `1.0887743028` | better |
| P99 ratio | `1.3784869434` | `1.2099676801` | better |
| C SQLite faster rows | `6` | `4` | better |
| FrankenSQLite faster rows | `18` | `16` | worse |

Large positive deltas were not enough to keep the change because the project
uses the primary weighted score as the insert keep gate. The candidate also
regressed important already-fast rows:

| Row | Baseline F ms | Candidate F ms | Delta ms |
| --- | ---: | ---: | ---: |
| `Single Transaction medium_6col / 10000 rows` | `3.686656` | `6.532769` | `+2.846113` |
| `Record Size large_10col / 10K rows` | `9.443103` | `10.459904` | `+1.016801` |
| `Single Transaction large_10col / 10000 rows` | `9.829345` | `10.713680` | `+0.884335` |
| `Transaction Strategy small_3col / 10000 single txn` | `2.093635` | `2.308167` | `+0.214532` |

The best improvements were mostly not enough to offset those regressions:

| Row | Baseline F ms | Candidate F ms | Delta ms |
| --- | ---: | ---: | ---: |
| `Record Size medium_6col / 10K rows` | `3.606074` | `3.386664` | `-0.219410` |
| `Transaction Strategy small_3col / 10000 autocommit` | `6.108546` | `5.920414` | `-0.188132` |
| `Single Transaction large_10col / 100 rows` | `0.210364` | `0.161953` | `-0.048411` |
| `Single Transaction medium_6col / 1000 rows` | `0.711200` | `0.683979` | `-0.027221` |

## Result

Rejected. Do not apply the scratch patch to the shared checkout.

Do not retry an already-dirty MemDatabase mirror abandon skip as a standalone
direct INSERT optimization. It is too small and noisy, and same-window focused
INSERT worsened the primary weighted score despite p90/p99 improvements.
Reconsider only if a broader direct INSERT execution-fusion design removes
row-build, MemDatabase mirror state, and page-run costs together and passes the
focused INSERT and full quick keep gates.

Ledger update was blocked by CrimsonGorge's active reservation on
`docs/progress/perf-negative-results.md`; a patch-ready entry was sent by Agent
Mail.
