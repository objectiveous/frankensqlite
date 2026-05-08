# Direct DELETE scratch-guard trim candidate

- Date: 2026-05-08
- Agent: SwiftGate
- Baseline: `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/update-profile.json`
- Candidate artifacts:
  - `candidate-update.json`
  - `candidate-update-repeat.json`
  - `candidate-update.html`
  - `candidate-update-repeat.html`
  - `stdout/candidate-update.stdout`
  - `stdout/candidate-update.stderr`
  - `stdout/candidate-update-repeat.stdout`
  - `stdout/candidate-update-repeat.stderr`

## Candidate

The rejected source patch removed the `StatementLookasideGrowthGuard` and
`PreparedDirectInsertScratchResetGuard` from
`execute_prepared_direct_simple_delete`. Direct DELETE does not use the prepared
direct INSERT/update scratch buffers, so the candidate tested whether skipping
those per-row guard costs could close the 100-row DELETE tail without touching
direct UPDATE or INSERT.

## Correctness Proofs Before Rejection

- `cargo fmt --check -p fsqlite-core`
- `cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`

## Benchmark Result

The first focused DML run showed a small 100-row DELETE absolute win, but lost
the section gate:

- Frontier update summary: faster/comparable/C-faster `4/0/2`, average ratio
  `1.022914`, geomean `1.005124`, median `0.897000`, p90 `1.368560`, p99
  `1.368560`, weighted score `1.005124`.
- Candidate first run: faster/comparable/C-faster `1/3/2`, average ratio
  `1.080121`, geomean `1.066384`, median `0.965554`, p90 `1.338856`, p99
  `1.338856`, weighted score `1.066384`.
- Candidate repeat: faster/comparable/C-faster `1/3/2`, average ratio
  `1.114058`, geomean `1.095675`, median `0.998102`, p90 `1.442364`, p99
  `1.442364`, weighted score `1.095675`.

Target rows:

- `100 rows / delete 5 rows`: frontier F `0.114064 ms`; first candidate F
  `0.109335 ms`; repeat candidate F `0.113553 ms`.
- `10000 rows / delete 500 rows`: frontier F `3.077094 ms`; first candidate F
  `3.310016 ms`; repeat candidate F `3.248581 ms`.

## Decision

Rejected. The tiny guard trim did not produce a stable target-row win and
regressed the focused update/delete distribution. The source patch was manually
unwound; these artifacts and the negative ledger entry are the durable result.
