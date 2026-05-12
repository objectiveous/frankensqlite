# Current INSERT Profile After DELETE Rescreen

- Date: 2026-05-12 13:50 UTC
- Commit: `54c3e5b332f3b5ac7469c7ea3e82b646baba02fe`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-insert-rescreen-54c3e5b CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_INSERT=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --no-html --json-out tests/artifacts/perf/codex-insert-current-profile-after-delete-rescreen-20260512T1545Z/insert-current.json`
- Validity: `git_dirty=false`, `benchmark_binary_older_than_git_head=false`.

## Summary

The focused INSERT matrix covers 25 scenarios:

| Metric | Value |
| --- | ---: |
| Franken faster / comparable / C faster | `17 / 2 / 6` |
| Average F/C | `0.850708164187678` |
| Geomean F/C | `0.8221663401332341` |
| Focused weighted score | `0.8078101733370312` |

Rows above `1.0x` F/C remain the 100-row fixed-cost family:

| F/C | Scenario | C median | F median | C CV | F CV |
| ---: | --- | ---: | ---: | ---: | ---: |
| `1.6177809368891563` | `single-txn tiny_1col 100 rows` | `0.073157ms` | `0.118352ms` | `12.09%` | `25.40%` |
| `1.1249551739251704` | `small_3col 100 rows / single txn` | `0.075291ms` | `0.084699ms` | `2.65%` | `4.76%` |
| `1.1200236339187482` | `single-txn large_10col 100 rows` | `0.147246ms` | `0.164919ms` | `6.18%` | `5.24%` |
| `1.1085828753402847` | `single-txn small_3col 100 rows` | `0.077876ms` | `0.086332ms` | `3.62%` | `6.26%` |
| `1.1020097870111285` | `small_3col 100 rows / batched (100/txn)` | `0.078267ms` | `0.086251ms` | `4.70%` | `6.05%` |
| `1.0565314595975726` | `single-txn medium_6col 100 rows` | `0.103323ms` | `0.109164ms` | `9.02%` | `5.47%` |

The `tiny_1col 100` row is noisy in this run (`25.40%` FrankenSQLite CV).
Earlier current runs put that row near parity (`1.0215x` in the focused
INSERT profile and `1.0124x` in the fullquick frontier), so it should not be
treated as a standalone source target without a cleaner reproduction.

The larger rows are still green:

| F/C | Scenario | C median | F median |
| ---: | --- | ---: | ---: |
| `0.9636128501019516` | `single-txn large_10col 10000 rows` | `9.171507ms` | `8.837782ms` |
| `0.7687948427091649` | `single-txn small_3col 10000 rows` | `3.427536ms` | `2.635072ms` |
| `0.6077744096798058` | `single-txn medium_6col 10000 rows` | `5.749041ms` | `3.494120ms` |
| `0.5782923016803438` | `single-txn tiny_1col 10000 rows` | `2.559000ms` | `1.479850ms` |

## Profile Notes

All profiled red rows stayed on the prepared direct INSERT fast path
(`fast == rows`, `slow=0`) and used one empty-root page-run flush. Representative
100-row counters:

| Scenario | Insert us | Row build ns | Preserialize ns | Direct flush ns | Background ns | Schema validation ns | Change tracking ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `tiny_1col 100` | `49.3` | `4751` | `0` | `3597` | `2903` | `3234` | `2364` |
| `small_3col 100` | `72.3` | `27700` | `21841` | `3707` | `2864` | `3146` | `2403` |

The profile reinforces the same boundary as the previous focused INSERT
artifact: the remaining stable gap is fixed per-statement and per-row ceremony
on tiny 100-row INSERT batches, not a missed page-run fast path.

## Decision

No source patch from this rescreen. Prior measured rejects already cover the
standalone serializer, row template, row-scratch, page-run threshold/arena,
empty-root builder, borrowed-record flush, direct page-image, parser wrapper,
background-check, schema shortcut, and setup-only variants. A keeper still
needs a broader fused row/body/page construction design that proves focused
INSERT wins and fullquick primary-score neutrality or better in the same
benchmark window.
