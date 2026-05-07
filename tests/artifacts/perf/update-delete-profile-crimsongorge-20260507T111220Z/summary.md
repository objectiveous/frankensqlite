# UPDATE/DELETE focused profile

## Context

- Date: 2026-05-07 11:12 UTC
- Local commit: `617b0b2dcf71eeff1f91c29a2c88956b375e2437`
- Note: this local commit was peer-owned and was ahead of `origin/main` at the time of profiling.
- Build target: `/data/tmp/frankensqlite-crimsongorge-btreeguard-target/release-perf`
- Comprehensive profile command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-crimsongorge-btreeguard-target/release-perf/comprehensive-bench \
  --quick \
  --filter update \
  --json-out tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/report-update-delete.json \
  --no-html
```

## Comprehensive Section Result

- Scenarios: 6
- FrankenSQLite faster / comparable / C SQLite faster: `0 / 2 / 4`
- Average ratio: `1.161809628557041`
- Geomean ratio: `1.1514568045449403`
- Median ratio: `1.1384892008417877`
- p90/p99 ratio: `1.5145515239810492`

| Scenario | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| `100 rows / update 10 rows` | `0.087173 ms` | `0.132028 ms` | `1.5145515239810492` |
| `100 rows / delete 5 rows` | `0.146465 ms` | `0.152265 ms` | `1.0395999044140238` |
| `1000 rows / update 100 rows` | `0.408304 ms` | `0.432750 ms` | `1.0598720561150514` |
| `1000 rows / delete 50 rows` | `0.381404 ms` | `0.400409 ms` | `1.0498290526580738` |
| `10000 rows / update 1000 rows` | `3.664678 ms` | `4.282235 ms` | `1.16851603333226` |
| `10000 rows / delete 500 rows` | `3.462631 ms` | `3.942168 ms` | `1.1384892008417877` |

## DML Counter Signal

The comprehensive section includes setup and prepopulation work. The DML profile confirms the UPDATE/DELETE statements are hitting the direct path, not the VDBE fallback:

- `fs_update_100`: `mutate_us=12.6`, `commit_us=6.1`, `direct_update=10`, `fast=10`, `slow=0`, `vdbe_opcodes=0`.
- `fs_delete_100`: `mutate_us=9.3`, `commit_us=5.5`, `direct_delete=5`, `fast=5`, `slow=0`, `vdbe_opcodes=0`.
- `fs_update_10000`: `mutate_us=1214.6`, `commit_us=177.6`, `direct_update=1000`, `fast=1000`, `slow=0`, `btree_payload_copy_calls=1000`, `btree_payload_copy_bytes=20889`.
- `fs_delete_10000`: `mutate_us=869.3`, `commit_us=208.7`, `direct_delete=500`, `fast=500`, `slow=0`.

## Isolated Mutation Result

The narrow `perf-update-delete` binary removes most repeated setup/populate ceremony and shows the true direct mutation loop is still slower than C SQLite:

| Workload | Rows | Iterations | F per row | C per row | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| update | 100 | 1000 | `644 ns` | `298 ns` | `2.16x` |
| delete | 100 | 1000 | `1120 ns` | `259 ns` | `4.32x` |
| update | 10000 | 200 | `856 ns` | `337 ns` | `2.54x` |
| delete | 10000 | 200 | `1638 ns` | `282 ns` | `5.81x` |

## Root Cause Hypothesis

The direct UPDATE/DELETE implementation is semantically on the fast path, but mechanically still pays row-at-a-time cursor and payload costs:

- `execute_prepared_direct_simple_update` creates a fresh `BtCursor` for each prepared statement execution, then seeks by rowid.
- The fixed-width REAL update path copies the whole payload into scratch to discover the target offset, patches 8 bytes in the scratch buffer, then calls whole-payload overwrite.
- `execute_prepared_direct_simple_delete` also creates a fresh cursor and seeks for each rowid before deleting.

For the benchmark's monotone rowid loops (`i * 10` and `i * 20`), this misses the high-EV shape: a retained direct-DML cursor/run that walks forward through the B-tree, patches fixed-width payload bytes in-place, and performs page-local delete compaction once per touched leaf instead of paying independent root-to-leaf work for each row.

## Rejected Follow-up: Standalone Fixed-width REAL Leaf Patch

I tried the smallest direct UPDATE kernel first: a B-tree primitive that parsed
the current leaf payload, verified the target column was a fixed-width REAL, and
patched only the 8 value bytes. Correctness gates passed, and the candidate did
remove the UPDATE payload-copy counters:

- Baseline `fs_update_10000`: `btree_payload_copy_calls=1000`,
  `btree_payload_copy_bytes=20889`.
- Candidate `fs_update_10000`: `btree_payload_copy_calls=0`,
  `btree_payload_copy_bytes=0`.

The benchmark rejected it:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Section geomean ratio | `1.1514568045449403` | `1.2399807521821862` |
| `100 rows / update 10 rows` | `0.132028 ms`, `1.5145515239810492` | `0.134542 ms`, `1.5662448632728374` |
| `10000 rows / update 1000 rows` | `4.282235 ms`, `1.16851603333226` | `4.337518 ms`, `1.173031026575702` |

The payload copy was not the dominant cost by itself. The candidate still paid
per-row cursor construction/root descent and added another B-tree primitive plus
record-header parse around a tiny copied payload. The source changes were
removed; `report-update-delete-candidate.json`, `stdout-candidate.txt`, and
`stderr-candidate.txt` are kept here as rejection evidence.

## Rejected Follow-up: Per-row Scratch Reset Removal

I also tried removing `PreparedDirectInsertScratchResetGuard` from direct
UPDATE/DELETE execution. The direct DML paths already clear their scratch
buffers at point of use, so this tested whether the per-row guard was just
ceremony. Correctness gates passed, but the focused benchmark rejected it:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Section geomean ratio | `1.1514568045449403` | `1.1827616752954908` |
| `100 rows / update 10 rows` | `0.132028 ms`, `1.5145515239810492` | `0.126537 ms`, `1.428247324935663` |
| `10000 rows / update 1000 rows` | `4.282235 ms`, `1.16851603333226` | `4.374428 ms`, `1.1500152479099848` |
| `10000 rows / delete 500 rows` | `3.942168 ms`, `1.1384892008417877` | `4.068265 ms`, `1.1455357987592527` |

The small update row improved, but the section aggregate and large rows moved
the wrong way. The source change was removed; `report-update-delete-scratchreset-candidate.json`,
`stdout-scratchreset-candidate.txt`, and `stderr-scratchreset-candidate.txt`
are kept here as rejection evidence.

## Next Lever

Do not spend this lane on VDBE dispatch or parser work; the counters already show those are bypassed. The next plausible step-change lever is a B-tree/core direct-DML retained cursor kernel:

1. Retain cursor position across monotone rowid executions in the active transaction and advance within the current leaf when possible.
2. Cache the fixed-width REAL payload offset only after cursor reuse removes the per-row seek cost.
3. Leaf-batched delete: when a run touches multiple cells on one leaf, compact the leaf once.

This needs `crates/fsqlite-core/src/connection.rs` and likely `crates/fsqlite-btree/src/cursor.rs`. A standalone payload-byte patch is now fenced in the negative-results ledger.
