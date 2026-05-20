# bd-db300.8.3 Partial Verification Note

- bead_id: `bd-db300.8.3`
- run_id: `bd-db300.8.3-20260520T042654Z-2319862`
- artifact_dir: `tests/artifacts/perf/bd-db300-8-3-single-writer-role-20260520T042653Z`
- verification_status: `partial`

## What Ran

Foreground command attempted with an outer timeout:

```bash
timeout 30m env \
  BEAD_ID=bd-db300.8.3 \
  ROW_IDS=mixed_read_write_c4 \
  FIXTURE_IDS=frankensqlite \
  PLACEMENT_PROFILE_IDS=recommended_pinned,adversarial_cross_node \
  STORAGE_PROFILE_IDS=file_backed,memory \
  WARMUP=0 \
  REPEAT=1 \
  OUTPUT_DIR=/data/projects/frankensqlite/tests/artifacts/perf/bd-db300-8-3-single-writer-role-20260520T042653Z \
  SINGLE_WRITER_VERIFY_TARGET_DIR=/tmp/fsqlite-h3-quick-check \
  RCH_TARGET_DIR=/tmp/rch_target_bd_db300_8_3 \
  bash scripts/verify_bd_db300_8_1_1_matched_artifact_packs.sh
```

The run entered the first matched-pack cell
`mixed_read_write_c4/frankensqlite/recommended_pinned/file_backed/sqlite_reference`
and remained in local `release-perf` compilation. It was stopped after the user
reported the verification suite had exceeded the local timebox. No completed
H3 benchmark row was produced in this run.

Captured partial files:

- `events.jsonl`
- `packs/.../sqlite_reference/stderr.log`
- `packs/.../sqlite_reference/stdout.log`
- `concurrent_mode_default_guard.txt`
- `single_writer_role.json`
- `single_writer_role.md`

## Available Single-Writer Evidence

The available comparison evidence comes from the completed H2 run at
`/tmp/bd-db300-8-2-20260519T233037Z`, rerun after the single-writer baton wait
fix. That run completed the same Track H matched-pack row family:

| placement_profile_id | storage_profile_id | SQLite ops/s | MVCC ops/s | single-writer ops/s | single/MVCC ratio | retry delta vs MVCC |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| adversarial_cross_node | file_backed | 11541.489403816287 | 3244.612803076646 | 2450.3354074238227 | 0.7552011768862947 | 6 |
| adversarial_cross_node | memory | 403088.46380971 | 45131.66938885853 | 46095.014731966716 | 1.0213452184719318 | 0 |
| recommended_pinned | file_backed | 11651.337681340692 | 3228.323033557014 | 2450.778202955119 | 0.7591490001094517 | 6 |
| recommended_pinned | memory | 423986.6189823049 | 40651.546730375776 | 41861.70733810613 | 1.0297691159393474 | 0 |

Against the H1.2 baseline report, single-writer absolute throughput improved in
all four cells:

| placement_profile_id | storage_profile_id | current single-writer ops/s | baseline single-writer ops/s | ops delta | status |
| --- | --- | ---: | ---: | ---: | --- |
| recommended_pinned | file_backed | 2450.778202955119 | 1768.9725087697254 | 681.8056941853936 | mixed |
| recommended_pinned | memory | 41861.70733810613 | 34202.90806805558 | 7658.799270050549 | mixed |
| adversarial_cross_node | file_backed | 2450.3354074238227 | 1073.3324965275413 | 1377.0029108962815 | improved_or_held |
| adversarial_cross_node | memory | 46095.014731966716 | 36519.915040069645 | 9575.09969189707 | mixed |

## Decision

Forced single-writer mode remains comparison or fallback evidence only. It is
useful for separating shared fixed engine cost from MVCC-specific concurrency
effects, and for diagnosing fallback behavior, but it is not the product
default and not the headline performance path.

The product default remains MVCC concurrent writers. The source guard in this
artifact confirms `concurrent_mode_default: RefCell::new(true)` at both current
constructor sites in `crates/fsqlite-core/src/connection.rs`.
