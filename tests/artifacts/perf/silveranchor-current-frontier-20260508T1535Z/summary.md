# Current Perf Frontier - SilverAnchor - 2026-05-08

## Scope

Current source head during this pass was `6217800520d14b6989b76c51bae9a0930bb08f25`
(`docs(perf): align README claims with current matrix`). The current benchmark
source artifact remains
`tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/`, measured
at `953959cbb2b495700c0737d155e6f7c84ce20acc`. `git diff --name-only
953959cb..HEAD` shows only AGENTS/README/beads and perf-artifact changes, so no
Rust source changed between the measured code and this pass.

I re-read the current matrix, the focused INSERT/DML profiles, the performance
negative-results ledger, and the B-tree/direct-DML/direct-INSERT code paths
before deciding whether to patch source.

## Current Matrix

`rusticgrove-full-quick-current-20260508T1510Z/full-quick.json`:

- Scenarios: `93`
- Faster/comparable/slower: `81/4/8`
- Average/geomean ratio: `0.4529512304` / `0.2630086347`
- Primary weighted score: `0.3412469584`
- p90/p99 ratio: `1.0160260530` / `1.3806107387`

Remaining C-faster rows are all write-side:

| Section | Scenario | C SQLite ms | FrankenSQLite ms | F/C |
|---|---|---:|---:|---:|
| UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.082654 | 0.114113 | 1.381x |
| UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.088315 | 0.117801 | 1.334x |
| INSERT single transaction large_10col | 10000 rows | 9.512429 | 12.058057 | 1.268x |
| INSERT single transaction small_3col | 100 rows | 0.079018 | 0.096541 | 1.222x |
| INSERT transaction strategy small_3col | 100 rows / batched | 0.077545 | 0.087173 | 1.124x |
| INSERT record size comparison | large_10col 10K rows | 10.303201 | 11.359228 | 1.102x |
| INSERT single transaction medium_6col | 100 rows | 0.102141 | 0.108654 | 1.064x |
| INSERT transaction strategy small_3col | 100 rows / single txn | 0.078467 | 0.083406 | 1.063x |
| INSERT single transaction large_10col | 100 rows | 0.161433 | 0.167494 | 1.038x |
| INSERT single transaction tiny_1col | 100 rows | 0.070011 | 0.071133 | 1.016x |

## Profile Reads

Focused DML profile:

- Update/delete section stayed slow only at the 100-row tail.
- `100 rows / update 10 rows`: setup `52.1 us`, begin `6.9 us`, prepare
  `13.2 us`, mutate `12.0 us`, commit `5.7 us`.
- `100 rows / delete 5 rows`: setup `56.2 us`, begin `5.0 us`, prepare
  `11.4 us`, mutate `8.3 us`, commit `5.2 us`.

Focused INSERT profile:

- Focused INSERT faster/comparable/slower: `17/2/6`.
- Focused INSERT average/geomean: `0.8031423735` / `0.7802739991`.
- Focused INSERT weighted score: `0.7888688743`.
- `large_10col` record-size 10K profile: row build `4.323 ms`, B-tree insert
  `0.872 ms`, commit roundtrip `2.519 ms`, page pool misses `2006`.

I also attempted a fresh `perf record` over the current release-perf benchmark
binary. It emitted binary data to the terminal and reported `(null)` for the
capture file, leaving no usable new `perf.data`; this pass therefore relies on
the built-in profile counters and existing committed artifacts.

## Source Decision

No source patch was attempted in this pass.

The small DML rows are monotone rowid loops, but the currently exposed B-tree
helpers (`table_advance_to`, same-size overwrite, and delete) still publish each
row mutation with `write_page_data`. A real same-leaf DML batch would need a
connection-visible staged-page contract across prepared-statement executions so
later statements and reads observe pending changes. A cursor-only or retained
seek-hint patch is fenced by prior negative results and would repeat rejected
work.

The old 10K batched small-row INSERT target is now green in the current matrix
(`0.675x` F/C), so the prior broad depth-2 right-edge page-builder candidate is
not a current target. It also previously regressed 100-row, autocommit, and
large-record rows that are now the remaining frontier.

Large-row INSERT still needs a true bulk page/record builder, not another
standalone micro-optimization. The ledger already fences standalone expression
specialization, concat text-piece transduction, direct retained-leaf writer
fusion, quick-balance row-payload transfer, parent/new-leaf ownership tweaks,
larger page-buffer recycle caps, and page-lease batch changes.

## Next Keep Gate

A keepable next patch should satisfy one of these shapes before touching the
full matrix:

- Shared fixed write setup: improve at least the 100-row INSERT tails and the
  100-row UPDATE/DELETE tails in the same A/B window, without increasing
  C-faster rows.
- DML leaf-run operator: prove an isolated same-leaf UPDATE/DELETE mutation
  win while preserving read-after-write visibility across statement boundaries,
  then run the focused update filter.
- Large-row page builder: reduce absolute FSQLite medians for both
  `large_10col` single-transaction 10K and record-size `large_10col` 10K before
  the full quick matrix. The patch must batch record/page construction and
  parent divider writes together; one-lever quick-balance or text-builder
  changes are already fenced.

