# Direct DELETE duplicate-check monotone candidate

Date: 2026-05-16

Candidate: skip `SmallVec::contains` in `TableLeafDeleteRun` while accepted cell
indices remain strictly increasing, then fall back to the existing scan after an
out-of-order delete.

Decision: rejected and runtime changes reverted. The fresh-eyes audit kept only
the new correctness test for out-of-order duplicate handling.

Command:

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-delete-dupcheck-monotone-candidate-20260516T1750Z/update-delete.json --no-html
```

RCH did not retain the raw JSON locally for this candidate, so the session
console output is the durable source for the measured rows below.

Compared with the previous kept compact-cache artifact
`tests/artifacts/perf/codex-delete-compact-cache-candidate-20260516T1721Z/summary.md`,
the target row regressed from `F=342.2 us` to `F=443.3 us`.

Measured rows:

| Scenario | C SQLite | FrankenSQLite | Result |
| --- | ---: | ---: | --- |
| 100 rows / update 10 rows | 6.0 us | 8.6 us | 1.44x slower |
| 100 rows / delete 5 rows | 3.3 us | 9.8 us | 2.98x slower |
| 1000 rows / update 100 rows | 56.0 us | 51.6 us | 1.09x faster |
| 1000 rows / delete 50 rows | 24.4 us | 48.2 us | 1.97x slower |
| 10000 rows / update 1000 rows | 559.9 us | 415.5 us | 1.35x faster |
| 10000 rows / delete 500 rows | 248.9 us | 443.3 us | 1.78x slower |

Target-row counters:

```text
fs_delete_10000 dml_profile:
mutate_us=543.8 commit_us=58.7 mutations=500 direct_delete=500
delete_leaf_start=64/67 delete_leaf_active=433/496 delete_leaf_miss=63
delete_leaf_flush=64/64 delete_leaf_flush_ns=81403
delete_leaf_materialize=64/61434 delete_leaf_write=64/11673
delete_leaf_search=560/63493 delete_leaf_dupcheck=500/18128
delete_leaf_compact=497/16795 delete_leaf_cellparse=497/19533
```

Root cause: the local duplicate-check counter improved relative to the noisy
previous candidate counter, but it was not the limiting cost in the matrix row.
The extra state/branch did not translate into end-to-end DELETE movement and
the measured target row moved materially backward.

Keep gate result: not a keep. Do not retry this as a standalone retained
leaf-run duplicate-check micro-optimization.
