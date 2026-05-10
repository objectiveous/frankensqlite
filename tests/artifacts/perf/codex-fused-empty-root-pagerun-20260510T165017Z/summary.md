# Fused empty-root page-run candidate

Date: 2026-05-10

Status: rejected and source patch reverted.

## Candidate

The candidate added a fused direct-insert page-run for empty-root tables that
packed large sorted rowid records directly into leaf page images while rows were
executed, instead of retaining one owned `Vec<u8>` per row and building pages at
flush time.

## Evidence

- Candidate JSON:
  `tests/artifacts/perf/codex-fused-empty-root-pagerun-20260510T165017Z/insert-quick.json`
- Baseline JSON:
  `tests/artifacts/perf/codex-fresh-frontier-insert-profile-20260510T093306Z/insert.json`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fused-pagerun-local-bench CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_INSERT=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-fused-empty-root-pagerun-20260510T165017Z/insert-quick.json --no-html`

## Result

The candidate was rejected. It did trigger on the intended large-record row
(`page_run_owned=0`, `page_run_arena=0`, `page_run_repeated=0`,
`page_run_empty_root=1` for `large_10col`), but it worsened the focused INSERT
matrix instead of improving it.

Selected same-window baseline-to-candidate FSQLite median deltas:

| Row | Baseline | Candidate | Delta |
| --- | ---: | ---: | ---: |
| record-size `large_10col` 10K | 10.1973 ms | 12.1962 ms | +19.6% |
| single-txn `large_10col` 1K | 0.8805 ms | 1.0009 ms | +13.7% |
| transaction strategy small 3col 1K single-txn | 0.2872 ms | 0.4345 ms | +51.3% |
| transaction strategy small 3col 100 batched | 0.0872 ms | 0.2165 ms | +148.4% |

Do not retry this fused leaf-page builder as a standalone optimization. The
page image construction work moved into the per-row execution path and lost more
than it saved at flush.
