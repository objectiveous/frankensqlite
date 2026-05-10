# Same-leaf DELETE next-cell hint candidate

Date: 2026-05-10

Candidate: track `TableLeafDeleteRun`'s last accepted cell index and check the
next cell before falling back to the existing binary search.

Baseline:
`tests/artifacts/perf/codex-current-dml-profile-20260510T165812Z/update-delete.json`

Candidate artifacts:

- `update-delete.json`
- `update-delete-repeat.json`

Proof before measurement:

- `cargo fmt -p fsqlite-btree --check`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hint-test CARGO_BUILD_JOBS=4 cargo test -p fsqlite-btree table_leaf_delete_run -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hint-test CARGO_BUILD_JOBS=4 cargo test -p fsqlite-core prepared_direct_delete_leaf_run -- --nocapture --test-threads=1`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hint-check CARGO_BUILD_JOBS=4 cargo check -p fsqlite-btree -p fsqlite-core --all-targets`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hint-check CARGO_BUILD_JOBS=4 cargo clippy -p fsqlite-btree -p fsqlite-core --all-targets -- -D warnings`

Result: rejected and source reverted.

The first run improved the focused write-single geomean from `1.4079632999` to
`1.3215911619`, but the key `10000 rows / delete 500 rows` FSQLite median
regressed from `0.368549 ms` to `0.392204 ms` (+6.4%), and the profile's active
delete-run time rose from `48.526 us` to `51.702 us`.

The repeat failed the keep gate: scenario counts moved to `1` faster / `5`
C-faster, geomean worsened to `1.6479249706`, `1000 rows / delete 50 rows`
regressed from `0.079258 ms` to `0.082975 ms` (+4.7%), and
`10000 rows / delete 500 rows` stayed regressed at `0.392675 ms` (+6.5%).
