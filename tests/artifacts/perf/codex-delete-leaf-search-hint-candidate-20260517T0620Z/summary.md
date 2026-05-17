# DELETE leaf-run search hint candidate

- Date: 2026-05-17
- Source: `6b4181415c1e1a38c013b895cdca5f8ace522aaa` plus dirty profiling-counter patch and the temporary `TableLeafDeleteRun` search-hint candidate.
- Candidate: cache the last verified `(rowid, cell_idx)` inside a retained table-leaf DELETE run, probe the predicted next dense rowid position, and fall back to the existing binary search on the same immutable leaf image.
- Correctness checks:
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hint-test-20260517 cargo test -p fsqlite-btree table_leaf_delete_run -- --nocapture`
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hint-check-20260517 cargo check -p fsqlite-btree`
- Profile artifact:
  - `tests/artifacts/perf/codex-delete-leaf-search-hint-candidate-20260517T0620Z/update-delete.json`
  - `tests/artifacts/perf/codex-delete-leaf-search-hint-candidate-20260517T0620Z/run.log`
- Non-profile artifacts:
  - `tests/artifacts/perf/codex-delete-leaf-search-hint-candidate-noprofile-20260517T0630Z/update-delete.json`
  - `tests/artifacts/perf/codex-delete-leaf-search-hint-candidate-noprofile-repeat-20260517T0635Z/update-delete.json`
  - `tests/artifacts/perf/codex-delete-leaf-search-hint-fullquick-20260517T0640Z/full-quick.json`
- Result: rejected and unwound. The profiled `10000 rows / delete 500 rows` search counter improved from the current baseline's `delete_leaf_search=560/40882` to `560/18049`, and the profiled F median moved from about `276.6 us` to `239.4 us`. The full quick matrix still rejected the patch: weighted score regressed from `0.365835734` to `0.380760547`, FrankenSQLite-faster rows fell from `80` to `77`, C-SQLite-faster rows rose from `10` to `13`, and the update-delete filter remained red. The full-matrix `10000 rows / delete 500 rows` F median moved only from `261.890 us` to `255.719 us`, while the ratio worsened from `1.194x` to `1.567x`.
- Retry condition: only revisit same-leaf search hints if the benchmark can isolate stable F-side DELETE movement without a full-matrix score regression. Prefer a broader materialization/flush or commit-path candidate over another rowid-probe hint.
