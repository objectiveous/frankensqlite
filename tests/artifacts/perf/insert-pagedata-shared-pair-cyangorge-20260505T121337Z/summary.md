# PageData shared-pair quick-balance candidate

Run: `2026-05-05T12:13:37Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --filter insert --json-out tests/artifacts/perf/insert-pagedata-shared-pair-cyangorge-20260505T121337Z/report.json --no-html
```

Baseline: `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/`

Candidate: `PageData::into_shared_pair()` consumed a newly built quick-balance
right-sibling page and returned two shared handles, avoiding the eager full-page
clone used by `PageData::clone()`.

Verdict: rejected and reverted.

The headline insert ratios improved (`per_category_weighted.score`
`1.7141 -> 1.6779`, geomean `2.3519x -> 2.2496x`), but target-row absolute
FrankenSQLite medians were mixed. The first run regressed the split-heavy
`large_10col` single-transaction rows: 10K `34.756 ms -> 37.418 ms`, 100K
`415.902 ms -> 444.273 ms`.

Root cause: the original clone-based handoff keeps the cursor's rightmost page
owned and mutable while giving the writer a shared snapshot. The shared-pair
handoff made the cursor's cached page shared too, so subsequent appends to the
same rightmost page pay copy-on-write.

