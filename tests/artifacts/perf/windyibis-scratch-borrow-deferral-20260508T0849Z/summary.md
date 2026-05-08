# Scratch-Borrow Deferral Candidate Recheck

Date: 2026-05-08
Agent: WindyIbis
HEAD: `ebb2345767d598f9ab679d8d6a0436f6914797c7`

## Scope

Measured an existing dirty `crates/fsqlite-core/src/connection.rs` diff that
defers `prepared_direct_update_row_scratch` borrowing until after the
fixed-width REAL direct-UPDATE overwrite path.

I did not edit, stage, or commit `connection.rs`: Agent Mail reported active
exclusive reservations for that file held by `BoldLion`. The measured source
diff is captured in `dirty-connection.diff`.

## Validation

- `cargo fmt -p fsqlite-core --check`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-scratch-borrow-deferral-test-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-scratch-borrow-deferral-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`

## Benchmark

Command:

```bash
/data/tmp/frankensqlite-windyibis-scratch-borrow-deferral-target/release-perf/comprehensive-bench --quick --filter UPDATE --json-out tests/artifacts/perf/windyibis-scratch-borrow-deferral-20260508T0849Z/candidate-update-delete.json --no-html
```

Baseline comparison uses the clean current-HEAD full quick artifact:
`tests/artifacts/perf/windyibis-medium100-current-profile-20260508T0835Z/current-full-quick.json`.

| Scenario | Baseline ratio | Candidate ratio | Candidate F median |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.400045 | 1.401359 | 0.118342 ms |
| 100 rows / delete 5 rows | 1.367213 | 1.383777 | 0.109966 ms |
| 1000 rows / update 100 rows | 0.984667 | 1.076061 | 0.418834 ms |
| 1000 rows / delete 50 rows | 0.955243 | 1.053299 | 0.383899 ms |
| 10000 rows / update 1000 rows | 0.838636 | 0.978240 | 3.541955 ms |
| 10000 rows / delete 500 rows | 0.829025 | 1.017410 | 3.378117 ms |

## Result

Rejected. The target 100-row UPDATE tail did not improve, and the 1000/10000
row UPDATE/DELETE ratios regressed versus the clean current-HEAD artifact. I did
not run a full quick matrix because the focused section failed the keep gate.
