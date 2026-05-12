# Retained DELETE Local-Payload Candidate

Date: 2026-05-12
Base commit: `a7635094b3701b0ac978aaf1bc0ee2a798571bee`
Candidate source: temporary `crates/fsqlite-btree/src/cursor.rs` patch, unwound before commit

Command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-delete-local-payload-candidate-20260512T2020Z/update-delete-profile.json \
  --no-html
```

Result: rejected. The successful-cell local-payload validation helper did not
produce a stable UPDATE/DELETE section win and was removed before commit.

Key DELETE rows:

| Scenario | Candidate F median | Candidate ratio |
| --- | ---: | ---: |
| 100 rows / delete 5 rows | 6.5 us | 2.92x slower |
| 1000 rows / delete 50 rows | 102.3 us | 1.54x slower |
| 10000 rows / delete 500 rows | 335.1 us | 1.48x slower |

The intended counter barely moved on the 10000-row row:
`delete_leaf_cellparse=497/12725ns` versus the baseline
`497/13074ns`, while the larger DELETE section rows regressed.
