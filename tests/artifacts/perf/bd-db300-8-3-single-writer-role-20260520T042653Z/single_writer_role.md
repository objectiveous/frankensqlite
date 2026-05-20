# H3 Single-Writer Role And Evidence

- bead_id: `bd-db300.8.3`
- verification_status: `partial`
- role: `comparison_or_fallback_only`
- product default: `fsqlite_mvcc`
- concurrent_mode_default guard: `passed`
- current partial artifact: `tests/artifacts/perf/bd-db300-8-3-single-writer-role-20260520T042653Z`
- completed evidence source: `/tmp/bd-db300-8-2-20260519T233037Z`

## Role

`BEGIN` stays MVCC-by-default through `concurrent_mode_default=true`. Forced
single-writer is opt-in via `PRAGMA fsqlite.concurrent_mode=OFF` or benchmark
`--no-mvcc`.

G4 should use forced single-writer as comparison or fallback evidence only. It
is a causal bridge between SQLite and MVCC: SQLite is the external baseline,
forced single-writer measures FrankenSQLite's shared fixed tax without MVCC
concurrency, and MVCC measures the intended concurrent-writer product path.

## Available Benchmark Evidence

| placement_profile_id | storage_profile_id | current single-writer ops/s | baseline single-writer ops/s | ops delta | single/MVCC ratio | retry delta vs MVCC | status |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| recommended_pinned | file_backed | 2450.778202955119 | 1768.9725087697254 | 681.8056941853936 | 0.7591490001094517 | 6 | mixed |
| recommended_pinned | memory | 41861.70733810613 | 34202.90806805558 | 7658.799270050549 | 1.0297691159393474 | 0 | mixed |
| adversarial_cross_node | file_backed | 2450.3354074238227 | 1073.3324965275413 | 1377.0029108962815 | 0.7552011768862947 | 6 | improved_or_held |
| adversarial_cross_node | memory | 46095.014731966716 | 36519.915040069645 | 9575.09969189707 | 1.0213452184719318 | 0 | mixed |

The H2 follow-up evidence shows shared single-writer-visible improvements in
absolute ops/s on all four cells. The mode still trails MVCC on file-backed
cells and is comparable or slightly ahead on memory cells, so it remains a
diagnostic/control lane rather than a headline performance mode.

## Verification Plan For G4

Unit tests:

- `cargo test -p fsqlite-e2e --test bd_2yqp6_6_5_concurrent_mode_defaults`
- `cargo test -p fsqlite-e2e --test bd_db300_7_1_2_counter_schema_alignment`

End-to-end scenarios:

- `realdb-e2e verify-suite --mode fsqlite_single_writer --verification-depth quick --activation-regime low_concurrency_fixed_cost --placement-profile baseline_unpinned`
- Matched packs for `sqlite_reference`, `fsqlite_mvcc`, and `fsqlite_single_writer` on `mixed_read_write_c4` with `recommended_pinned` and `adversarial_cross_node` placement metadata.

Logging artifacts:

- `events.jsonl`
- `partial_verification_note.md`
- `single_writer_role.json`
- `single_writer_role.md`
- `concurrent_mode_default_guard.txt`
