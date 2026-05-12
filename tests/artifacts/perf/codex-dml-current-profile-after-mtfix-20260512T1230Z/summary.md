# Current DML Profile After Shared-Table Retry Fix

- Date: 2026-05-12 12:21 UTC
- Commit: `78728d45b5efa5c6cfbbcb2edd9f8d57f89b41e3`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-profile-78728d45 CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html --json-out tests/artifacts/perf/codex-dml-current-profile-after-mtfix-20260512T1230Z/update-delete-current.json`
- Validity: `git_dirty=false`, `benchmark_binary_older_than_git_head=false`.

## Ratios

| F/C | Scenario |
| ---: | --- |
| `3.120766488413547` | `100 rows / delete 5 rows` |
| `2.3492762743864066` | `1000 rows / delete 50 rows` |
| `1.673670444100356` | `10000 rows / delete 500 rows` |
| `1.3738339552238805` | `100 rows / update 10 rows` |
| `0.7558884772438411` | `1000 rows / update 100 rows` |
| `0.6195037473852396` | `10000 rows / update 1000 rows` |

## Hot-Path Counters

The profile keeps the current DELETE workload entirely on the direct fast path:

| Scenario | Fast/slow | Leaf starts | Active hits | Flushes | Materialize ns | Write ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `delete 5/100` | `5/0` | `1/1` | `4/4` | `1/1` | `1192` | `321` |
| `delete 50/1000` | `50/0` | `6/6` | `44/49` | `6/6` | `4287` | `1073` |
| `delete 500/10000` | `500/0` | `64/67` | `433/496` | `64/64` | `39044` | `7902` |

For `delete 500/10000`, the remaining split is still distributed across row existence checks and retained-run publication: `delete_seek_ns=33931`, `delete_leaf_active_ns=49953`, `delete_leaf_flush_ns=52448`, and `commit_roundtrip_ns=20799`.

## Decision

No source patch from this profile. A pure logical-delete staging shortcut would still have to return each statement's affected-row count immediately, so it cannot skip the row-existence work without a read-view/affected-row overlay. It would also need to flush before any read, insert, update, savepoint, or incompatible write boundary. That is the same broader transaction-local DML mutation/read-view operator already identified in the negative ledger, not a safe standalone micro-optimization.
