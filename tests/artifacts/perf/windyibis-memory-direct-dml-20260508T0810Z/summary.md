# Private-Memory Direct DML Page-IO Bypass Recheck

Date: 2026-05-08
Agent: WindyIbis
Source commit during recheck: `d385b0ed8f33755e8c01c7480ab6529125144c56`
Git dirty in reports: `true`

## Context

This rechecked the private `:memory:` direct UPDATE/DELETE `SharedTxnPageIo`
bypass candidate that had already been rejected and recorded in
`docs/progress/perf-negative-results.md` by commit `d385b0ed8f33755e8c01c7480ab6529125144c56`.

The temporary source diff added `direct_update_delete_page_io_context()` and
returned `None` for private in-memory databases, so direct UPDATE/DELETE used
the active transaction cursor instead of constructing `SharedTxnPageIo`. The
source candidate was restored after this recheck; no source change was kept.

## Correctness Checks

Before discovering the existing rejection commit, the temporary candidate passed:

```text
cargo fmt -p fsqlite-core --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-memory-direct-dml-test-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-memory-direct-dml-test-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_commit_without_writes_clears_concurrent_session_state -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-memory-direct-dml-test-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_begin_without_mode_creates_mvcc_session_when_default_on -- --nocapture
```

The release-perf candidate build completed locally after RCH timed out:

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-memory-direct-dml-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin perf-update-delete --bin comprehensive-bench --profile release-perf
```

## Isolated Probe

`perf-update-delete 10000 30 both compare isolated` improved FSQLITE per-row
mutation time versus the pre-candidate probe noted in the session handoff:

| Shape | Before | Recheck candidate |
| --- | ---: | ---: |
| Update ratio | `2.70x` | `2.30x` |
| Delete ratio | `4.07x` | `4.11x` |
| FSQLITE per-row update | `968 ns` | `762 ns` |
| FSQLITE per-row delete | `1169 ns` | `1041 ns` |

Additional isolated probes:

| Shape | Candidate update ratio | Candidate delete ratio |
| --- | ---: | ---: |
| 100 rows, 200 iters | `2.08x` | `3.91x` |
| 1000 rows, 100 iters | `2.05x` | `3.39x` |

## Focused UPDATE/DELETE Recheck

Artifacts:

- `baseline-update.json`: same-window run with the pre-candidate binary.
- `candidate-update.json`: same-window run with the temporary bypass.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Average ratio | `1.1715454479` | `1.0747467202` |
| Geomean ratio | `1.1492035553` | `1.0604371744` |
| P90/P99 ratio | `1.5828996669` | `1.3690457537` |
| Faster / comparable / slower | `1 / 2 / 3` | `2 / 2 / 2` |

Rows:

| Row | Baseline ratio | Candidate ratio | Candidate F CV% |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | `1.582900` | `1.278033` | `5.53` |
| 100 rows / delete 5 rows | `1.312588` | `1.369046` | `3.84` |
| 1000 rows / update 100 rows | `1.249651` | `1.015942` | `12.93` |
| 1000 rows / delete 50 rows | `0.951960` | `0.951741` | `1.99` |
| 10000 rows / update 1000 rows | `1.003158` | `0.907686` | `8.84` |
| 10000 rows / delete 500 rows | `0.929016` | `0.926033` | `5.79` |

## Decision

Rejected as a standalone source change, matching the existing negative-ledger
entry. The recheck improved focused DML average/geomean and the 100-row update
tail, but it still worsened the 100-row delete row and does not satisfy the
ledger's retry condition that both 100-row update/delete tails improve before a
full quick keep gate.

Future work should follow the ledger's guidance: revisit this only inside a
broader batch/leaf-run DML operator that reduces setup and mutation work
together, then require repeated focused UPDATE/DELETE gates and a full quick
matrix.
