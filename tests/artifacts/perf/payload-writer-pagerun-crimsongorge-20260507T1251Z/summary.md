# Payload-Writer Non-Empty Page-Run Candidate - 2026-05-07

Agent: CrimsonGorge

This was a read-only evaluation of the dirty `crates/fsqlite-core/src/connection.rs`
candidate in the shared worktree. PurpleOtter held the file reservation, so this
pass did not edit, stage, or revert source code.

Candidate shape:

- Replay pending direct INSERT page-run records through
  `table_append_after_last_position_with_writer`, falling back to
  `table_append_after_last_position` if the writer path cannot append.
- Allow non-empty right-edge page-run buffering when the new explicit rowid is
  greater than the current last rowid.
- Update the savepoint-boundary test expectation so post-savepoint non-empty
  right-edge inserts may buffer until rollback or release.

## Proof Commands

```bash
git diff --check -- crates/fsqlite-core/src/connection.rs
rch exec -- env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-payload-writer-pagerun-target \
  CARGO_BUILD_JOBS=16 \
  cargo fmt -p fsqlite-core --check
rch exec -- env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-payload-writer-pagerun-target \
  CARGO_BUILD_JOBS=16 \
  cargo test -p fsqlite-core prepared_direct_insert_page_run -- --nocapture
rch exec -- env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-payload-writer-pagerun-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
/data/tmp/frankensqlite-payload-writer-pagerun-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/payload-writer-pagerun-crimsongorge-20260507T1251Z/candidate-insert.json \
  --no-html
```

## Correctness Result

- `git diff --check -- crates/fsqlite-core/src/connection.rs`: passed.
- `cargo fmt -p fsqlite-core --check`: passed.
- `cargo test -p fsqlite-core prepared_direct_insert_page_run`: passed, 3 tests.
- `cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`:
  passed.

## Benchmark Result

Baseline comparison uses the full quick refresh published in
`tests/artifacts/perf/full-refresh-crimsongorge-20260507T1246Z/report-full.json`
from clean git `5b36871d3fb29b9728ec9e34c64a8c21b45151f1`. This is close in
time but not a same-window rebuild; the result is enough to reject because the
candidate misses its target row.

| Row | Baseline F ms | Candidate F ms | Baseline ratio | Candidate ratio |
| --- | ---: | ---: | ---: | ---: |
| 10000 rows / batched (1000/txn) | 4.281322 | 4.375808 | 1.2771 | 1.3310 |
| 100 rows / single txn | 0.094778 | 0.089838 | 1.1518 | 1.2142 |
| large_10col 100 rows | 0.207679 | 0.195295 | 1.3763 | 1.1012 |
| large_10col 10K record-size | 10.967841 | 10.632109 | 1.1331 | 1.0671 |

The candidate improved some large-record rows in absolute FSQLite time, but the
primary target for the non-empty page-run idea, `10000 rows / batched
(1000/txn)`, regressed by about 2.2% in FSQLite median and worsened relative to
C SQLite. The smaller transaction rows also remained C-faster.

## Decision

Rejected as a standalone keep. Replaying a non-empty page-run through the
payload-writer append path is still row-at-a-time work and does not fix the
batched append gap. The next viable retry must build pages/runs directly enough
to remove repeated cursor/page append ceremony, not only swap the replay kernel.
