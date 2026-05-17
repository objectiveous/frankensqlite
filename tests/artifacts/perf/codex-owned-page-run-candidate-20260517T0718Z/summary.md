# Owned page-run record move candidate - 2026-05-17T0718Z

## Candidate

Move the preserialized large-record scratch `Vec<u8>` into already-active
`PendingDirectInsertPageRunRecords::Owned` runs instead of cloning the record
bytes into the run. The candidate kept the existing page-run admission gates,
rowid monotonicity checks, savepoint boundaries, and bulk-loader selection.

## Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-owned-page-run-candidate-20260517 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-owned-page-run-candidate-20260517T0718Z/insert.json --no-html
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-owned-page-run-candidate-20260517 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-owned-page-run-candidate-repeat-20260517T0725Z/insert.json --no-html
```

RCH reported no admissible workers and ran locally.

## Result

Rejected and unwound uncommitted.

First run:

- FrankenSQLite faster / comparable / C SQLite faster: `16 / 2 / 7`
- Average / geomean F/C ratio: `1.008 / 0.943`
- P90 / P99 F/C ratio: `1.449 / 2.412`
- Weighted score: `0.828`
- Target rows: large 10K single transaction `1.066x` slower; record-size
  large row `1.037x` slower.

Repeat:

- FrankenSQLite faster / comparable / C SQLite faster: `14 / 3 / 8`
- Average / geomean F/C ratio: `0.938 / 0.914`
- P90 / P99 F/C ratio: `1.199 / 1.353`
- Weighted score: `0.967`
- Target rows: large 10K single transaction `1.155x` slower; record-size
  large row `1.092x` slower.

The repeat failed the keep gate against the current no-profile repeat baseline
(`weighted=0.934`, large 10K single transaction `1.159x`, record-size large
`1.135x`). The candidate improved one large record-size row but did not
reliably move the large 10K row and made the insert section score worse.

## Decision

Do not retry moving the reusable preserialized record scratch into active owned
page-runs as a standalone optimization. The duplicate copy is not enough on its
own; revisit only inside a true fused row/body/page construction design with a
full focused INSERT and full quick keep gate.
