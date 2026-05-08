# Page-one owned staging candidate

Date: 2026-05-08
Agent: SilverAnchor
Source revision: `36014ec4f6e2bca1515dace9c52e607e17772dc1` plus local `crates/fsqlite-pager/src/pager.rs`

## Scope

Targeted the concurrent-writer profile path where `PageBufPool::acquire` and a page-sized copy appeared under `ensure_page_one_in_write_set`.

The candidate keeps page 1 as a `StagedPage` directly. If page 1 is already staged, it is reused without converting through a pool buffer. If page 1 is read from storage, the owned `Vec<u8>` returned by `read_page_copy` is wrapped as `PageData` instead of copied into a newly acquired `PageBuf`.

## Focused results

Local baseline worktree: `/data/tmp/frankensqlite-silveranchor-page1-baseline-36014ec4`

Command shape:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-baseline-target CARGO_BUILD_JOBS=12 cargo run -p fsqlite-e2e --bin comprehensive-bench --profile release-perf -- --quick --filter concurrent --no-html --json-stdout
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-target CARGO_BUILD_JOBS=12 cargo run -p fsqlite-e2e --bin comprehensive-bench --profile release-perf -- --quick --filter concurrent --no-html --json-stdout
```

FSQLite median milliseconds from the initial paired local window:

| Scenario | Baseline r1 | Baseline r2 | Candidate r1 | Candidate r2 |
| --- | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 16.665723 | 15.069283 | 14.899987 | 14.968675 |
| 4 writers x 1000 rows | 22.210899 | 22.083861 | 20.759551 | 21.387747 |
| 8 writers x 1000 rows | 37.021888 | 38.046387 | 36.171295 | 36.306589 |

Focused geomean ratio improved from baseline `0.815115` / `0.837761` to candidate `0.758795` / `0.743826`.

After a small source cleanup, two additional warm candidate repeats landed in a noisier window:

| Scenario | Candidate post-cleanup r3 | Candidate post-cleanup r4 | Same-window baseline r3 |
| --- | ---: | ---: | ---: |
| 2 writers x 1000 rows | 16.777453 | 19.718311 | 16.638052 |
| 4 writers x 1000 rows | 24.574096 | 27.008716 | 27.764291 |
| 8 writers x 1000 rows | 41.897251 | 53.462672 | 52.405011 |

The focused microbench is therefore treated as a noisy directional signal rather than the final keep gate. The full quick matrix below is the final-source keep gate.

## Full quick matrix

Command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-target CARGO_BUILD_JOBS=12 cargo run -p fsqlite-e2e --bin comprehensive-bench --profile release-perf -- --quick --no-html --json-stdout
```

Final-source candidate: `candidate-local-full-quick-postcleanup.json`

Compared with `tests/artifacts/perf/boldlion-current-full-frontier-20260508T1020Z/current-full-quick.json`:

| Metric | Current frontier | Candidate |
| --- | ---: | ---: |
| total scenarios | 93 | 93 |
| faster / comparable / C faster | 83 / 3 / 7 | 80 / 4 / 9 |
| average ratio | 0.461816 | 0.463056 |
| geomean ratio | 0.273855 | 0.272121 |
| p90 ratio | 0.997851 | 1.037461 |
| p99 ratio | 1.525841 | 1.355334 |
| weighted score | 0.355358 | 0.353504 |

No scenario ratio worsened by more than 5% versus the current clean frontier in the JSON comparison.

## Noisy RCH samples

The initial RCH candidate samples on worker `ts2` were not used for the keep decision because they showed extreme host noise: C SQLite was hundreds of milliseconds to seconds on the concurrent rows. Local baseline and local candidate runs above are the comparable evidence.

## Verification

Passed:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-target CARGO_BUILD_JOBS=12 cargo check -p fsqlite-pager --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-target CARGO_BUILD_JOBS=12 cargo test -p fsqlite-pager page_one -- --nocapture
cargo fmt --check
git diff --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-check-local-target CARGO_BUILD_JOBS=12 cargo check --workspace --all-targets
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-page1-check-local-target CARGO_BUILD_JOBS=12 cargo clippy --workspace --all-targets -- -D warnings
```

UBS was run on `crates/fsqlite-pager/src/pager.rs` and exited 1 on existing file-wide inventories such as unwraps, test panics, asserts, and direct indexing in pre-existing lines. No UBS finding was identified as specific to this patch.

An RCH workspace check failed once with a stale-looking unresolved import for `commit_phase_timing_forced_enabled`; the same `fsqlite-e2e` import passed locally, and the full local workspace check passed.

## Decision

Keep, but as a small measured win. The focused concurrent rows are noisy after the final cleanup, while the final full quick matrix still improves the weighted score and geomean without a detectable scenario ratio regression over 5%.
