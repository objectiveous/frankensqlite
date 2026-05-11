# Current INSERT Profile After Memory Page-I/O Skip

- Date: 2026-05-11
- Commit: `e4aa479374d01efb079a4e9388bc1893510290d8`
- Command:
  `FSQLITE_BENCH_PROFILE_INSERT=1 comprehensive-bench --quick --filter insert --json-out insert-profile.json --no-html`
- Files: `insert-profile.json`, `insert-profile.stdout`

## Summary

The clean focused INSERT rerun reported `25` scenarios: `18`
FSQLite-faster, `2` comparable, and `5` C-SQLite-faster. Average ratio was
`0.8377801794`, geomean `0.8170096908`, median `0.8091892416`, p90
`1.1306435102`, p99 `1.1411721601`, and focused primary weighted score
`0.8191484915`.

The remaining red rows are:

- `small_3col` 100 rows: `1.12487x`
- `medium_6col` 100 rows: `1.06507x`
- `large_10col` 100 rows: `1.13064x`
- `large_10col` 10000 rows: `1.04768x`
- small 100-row batched/single-txn: `1.13442x` / `1.14117x`
- record-size `large_10col` 10K: `1.01376x`

## Attribution

Every profiled row stayed on the prepared direct INSERT lane:
`direct_insert == fast`, `slow=0`.

The 100-row red rows are fixed-cost dominated:

- `small_3col` 100: row-build `28083 ns`, direct flush `3557 ns`
- `medium_6col` 100: row-build `36023 ns`, direct flush `7414 ns`
- `large_10col` 100: row-build `56676 ns`, direct flush `17663 ns`

The large-record 10K rows still point at record construction plus owned
empty-root page-run publication:

- single-txn `large_10col` 10K: row-build `5665487 ns`, preserialize
  `5081633 ns`, direct flush `2202683 ns`
- record-size `large_10col` 10K: row-build `5506717 ns`, preserialize
  `4888437 ns`, direct flush `1892302 ns`

## Decision

No source patch was attempted. The source path is the same boundary already
fenced in the negative-results ledger: standalone serializer tweaks, row-build
templates, page-run threshold/arena variants, prebuilt empty-root builders, and
the direct fused page-image builder are not credible next standalone levers from
this evidence. A retry needs a broader fused row/body/page construction design
that moves work off the per-row execution path and wins the focused INSERT
primary score plus the full-quick matrix in the same measurement window.
