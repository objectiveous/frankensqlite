# Current HEAD Full Quick Check

Date: 2026-05-08
Agent: WindyIbis
Commit under test: `0f1b85ebafc5727b3ae010cfb64571bb134fb0af`

## Command

- `.rch-target/release-perf/comprehensive-bench --quick --no-html --json-out tests/artifacts/perf/windyibis-head-0f1b85eb-full-quick-20260508T0700Z/full-quick.json`

The harness reported `Git: main @ 0f1b85ebafc5727b3ae010cfb64571bb134fb0af`
and warned that the benchmark binary predates Git HEAD. The source files touched
by `41a950b6` and `0f1b85eb` had mtimes before the release-perf binary mtime, so
this is useful supporting evidence, but the focused INSERT profile remains the
primary rejection artifact for the 2048-cap candidate.

## Prior Keeper

Artifact:
`tests/artifacts/perf/windyibis-schema-index-plan-20260508T055049Z/pagebuf-full-quick.json`

- Weighted score: `0.3358994390491727`
- Average ratio: `0.4420352710879217`
- Geomean ratio: `0.2593408377033597`
- P90 ratio: `1.0575874643185499`
- P99 ratio: `1.2422341250364553`
- Faster/comparable/slower: `81 / 2 / 10`

## Current HEAD

Artifact:
`tests/artifacts/perf/windyibis-head-0f1b85eb-full-quick-20260508T0700Z/full-quick.json`

- Weighted score: `0.3552972206567397`
- Average ratio: `0.49071440245809644`
- Geomean ratio: `0.2786380467473357`
- P90 ratio: `1.0837347764642427`
- P99 ratio: `2.2697406591196656`
- Faster/comparable/slower: `79 / 3 / 11`

Worst row:

- `INSERTThroughput - Record Size Comparison (10K rows, single txn) / large_10col`
- C SQLite median: `8.950575 ms`
- FrankenSQLite median: `20.315484 ms`
- Ratio: `2.2697406591196656`

## Decision

Not a keeper as measured. This run supports the focused INSERT rejection of the
2048-cap page-buffer candidate and shows the current landed state is worse than
the prior keeper matrix on the primary weighted score and p99 guard.
