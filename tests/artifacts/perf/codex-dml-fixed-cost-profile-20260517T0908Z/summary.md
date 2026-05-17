# DML Fixed-Cost Profile Refresh

- Date: 2026-05-17 09:12 UTC.
- Source: `main` at `6b4181415c1e1a38c013b895cdca5f8ace522aaa` plus dirty
  profiling counters for direct DELETE fixed costs.
- Captured profiled run:
  `FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-dml-fixed-cost-profile-20260517T0908Z/update-delete-captured.json --no-html`
- Captured non-profile sanity run:
  `cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-dml-fixed-cost-profile-20260517T0908Z/update-delete-noprofile.json --no-html`

## Matrix Summary

- Profiled run: 6 scenarios, FrankenSQLite faster / comparable /
  C-SQLite-faster = 2 / 0 / 4, average ratio `1.606x`, geomean `1.430x`.
- Non-profile sanity run: 6 scenarios, FrankenSQLite faster / comparable /
  C-SQLite-faster = 2 / 0 / 4, average ratio `1.627x`, geomean `1.442x`.
- Non-profile red DELETE rows:
  - 100 rows / delete 5 rows: C `2.3 us`, F `7.4 us`, ratio `3.19x`.
  - 1000 rows / delete 50 rows: C `16.0 us`, F `29.4 us`, ratio `1.84x`.
  - 10000 rows / delete 500 rows: C `160.6 us`, F `255.5 us`, ratio `1.59x`.

## Direct DELETE Profile

Captured profile counters for DELETE rows:

| Row | Mutations | preflush ns | rowid ns | active-probe ns | cursor ns | memdb abandon | memory sync | seek ns | leaf flush ns | leaf search ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 delete | 5 | 171 | 140 | 1,664 | 60 | 5 / 140 | 5 / 362 | 1,072 | 1,673 | 430 |
| 1000 delete | 50 | 1,442 | 1,363 | 19,206 | 240 | 50 / 1,303 | 50 / 1,635 | 4,319 | 6,412 | 4,856 |
| 10000 delete | 500 | 13,718 | 13,527 | 232,551 | 2,529 | 500 / 13,167 | 500 / 14,010 | 48,583 | 56,475 | 43,203 |

Interpretation: the newly exposed fixed-cost pieces are measurable but not the
next source target. Even on the 10K DELETE row, preflush + rowid coercion +
MemDatabase invalidation + synced-root checks total roughly `54 us`, while the
retained leaf-run probe, seek/search, flush/materialization, and commit/publish
path remain larger. This supports keeping source work at the transaction-local
DML mutation-operator boundary rather than another standalone synced-root,
MemDatabase invalidation, or retained-leaf micro-patch.
