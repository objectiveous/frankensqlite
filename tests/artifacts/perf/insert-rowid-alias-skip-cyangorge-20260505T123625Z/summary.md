# Direct INSERT rowid-alias skip candidate

Run: `2026-05-05T12:36:25Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-rowid-alias-skip-cyangorge-20260505T123625Z/report.json --no-html
```

Baseline: `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/`

Candidate: skip re-evaluating the compiled INTEGER PRIMARY KEY alias expression
inside the direct-insert row-build loop after explicit rowid evaluation had
already run.

Verdict: rejected and reverted.

Results:

- `geomean_ratio`: `2.3623x -> 2.4502x`
- `per_category_weighted.score`: `1.6991 -> 1.7605`
- `p99_ratio`: `4.1407x -> 4.3519x`
- `large_10col` single transaction 10K: `36.165 ms -> 35.335 ms`
- record-size `large_10col` 10K: `37.056 ms -> 37.477 ms`

The standalone skip is too small and noisy relative to generated text building,
B-tree work, and commit publication.
