# Disable delete leaf-run candidate

Date: 2026-05-10

Status: rejected and source patch reverted.

## Candidate

The candidate disabled `can_defer_prepared_direct_delete_leaf_run` to test
whether the retained same-leaf delete-run machinery was net negative for the
small DELETE rows.

## Evidence

- Candidate JSON:
  `tests/artifacts/perf/codex-disable-delete-leaf-run-20260510T170227Z/update-delete.json`
- Baseline JSON:
  `tests/artifacts/perf/codex-current-dml-profile-20260510T165812Z/update-delete.json`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fused-pagerun-local-bench CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --json-out tests/artifacts/perf/codex-disable-delete-leaf-run-20260510T170227Z/update-delete.json --no-html`

## Result

Rejected. Disabling the delete leaf-run did not fix the small DELETE row and
badly regressed larger DELETE rows.

Selected FSQLite median deltas versus the same-session baseline:

| Row | Baseline | Candidate | Delta |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 23.293 us | 22.2 us | -4.7% |
| 1000 rows / delete 50 rows | 79.258 us | 107.1 us | +35.1% |
| 10000 rows / delete 500 rows | 368.549 us | 993.2 us | +169.5% |

Do not disable the retained delete leaf-run globally. Future DELETE work should
preserve the large-row retained-run win and target smaller overheads inside the
run or a narrow small-delete special case.
