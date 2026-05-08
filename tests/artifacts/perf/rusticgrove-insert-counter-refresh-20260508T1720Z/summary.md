# Focused INSERT Counter Refresh - 2026-05-08

## Source Basis

- Checkout: `/data/projects/frankensqlite`
- Git HEAD: `6fb45e85 docs(perf): refresh dml compare profile`
- Branch: `main`
- Source status before measurement: clean, `main` ahead of `origin/main` by
  one commit.
- Build target: `/data/tmp/frankensqlite-rusticgrove-main-target`
- CASS status: stale lexical index, semantic unavailable. Project-scoped
  searches for the current insert counter terms returned no direct hits; broad
  searches returned unrelated archived Gemini material. The negative ledger and
  current artifacts are the authority for no-retry decisions.

## Commands

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-main-target \
  CARGO_BUILD_JOBS=12 \
  cargo build --profile release-perf \
  -p fsqlite-e2e \
  --bin comprehensive-bench
```

Result: passed.

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-rusticgrove-main-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/rusticgrove-insert-counter-refresh-20260508T1720Z/insert-profile.json \
  --html tests/artifacts/perf/rusticgrove-insert-counter-refresh-20260508T1720Z/insert-profile.html
```

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-rusticgrove-main-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/rusticgrove-insert-counter-refresh-20260508T1720Z/insert-profile-repeat.json \
  --html tests/artifacts/perf/rusticgrove-insert-counter-refresh-20260508T1720Z/insert-profile-repeat.html
```

## Summary Metrics

The repeated focused insert run did not reproduce one first-run outlier
(`medium_6col` 1000 rows at `1.2757x`). The stable remaining slower cells are
small 100-row fixed-cost rows plus the large 10K record rows.

| Run | Faster / Comparable / C-faster | Avg | Geomean | Median | P90 | P99 | Weighted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insert-profile.json` | `16 / 1 / 8` | `0.892736` | `0.867378` | `0.880521` | `1.167188` | `1.275659` | `0.834653` |
| `insert-profile-repeat.json` | `17 / 0 / 8` | `0.846305` | `0.820502` | `0.809259` | `1.123659` | `1.160985` | `0.795684` |

Repeat rows above `1.0x`:

| Ratio | Scenario | C ms | F ms | C CV | F CV |
| ---: | --- | ---: | ---: | ---: | ---: |
| `1.1610` | single txn `large_10col` 100 rows | `0.145753` | `0.169217` | `4.6` | `6.2` |
| `1.1402` | single txn `medium_6col` 100 rows | `0.102362` | `0.116718` | `9.7` | `9.3` |
| `1.1237` | small_3col 100 rows / batched | `0.076323` | `0.085761` | `3.9` | `2.6` |
| `1.1196` | single txn `small_3col` 100 rows | `0.079239` | `0.088717` | `4.0` | `4.6` |
| `1.1128` | small_3col 100 rows / single txn | `0.076263` | `0.084869` | `4.3` | `5.3` |
| `1.1112` | single txn `tiny_1col` 100 rows | `0.066465` | `0.073858` | `16.0` | `8.6` |
| `1.0857` | single txn `large_10col` 10K rows | `8.982859` | `9.752851` | `1.1` | `12.1` |
| `1.0722` | record-size `large_10col` 10K rows | `9.520756` | `10.207763` | `2.7` | `3.7` |

## Counter Readout

Representative repeat `FSQLITE_BENCH_PROFILE_INSERT` counters:

| Row | setup_us | begin_us | prepare_us | insert_us | commit_us | row_build_us | btree_insert_us | schema_validation_us | page_pool_misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `tiny_1col` 100 | `16.6` | `10.3` | `8.3` | `48.0` | `11.1` | `3.4` | `3.2` | `3.2` | `1` |
| `small_3col` 100 | `16.8` | `10.0` | `9.8` | `58.0` | `10.8` | `13.4` | `3.3` | `3.2` | `1` |
| `medium_6col` 100 | `16.8` | `10.0` | `11.8` | `65.2` | `19.8` | `20.4` | `3.5` | `3.2` | `7` |
| `large_10col` 100 | `18.5` | `10.0` | `17.1` | `91.7` | `40.4` | `44.4` | `5.9` | `3.2` | `22` |
| `large_10col` 10K | `40.3` | `14.4` | `33.6` | `8906.0` | `4800.7` | `4278.3` | `654.6` | `311.1` | `2006` |
| record-size `large_10col` 10K | `36.3` | `13.6` | `29.8` | `9210.9` | `4202.5` | `4454.1` | `737.4` | `311.0` | `2006` |

Interpretation:

- 100-row gaps are mostly fixed ceremony plus record-size-sensitive row
  building. The visible per-row schema validation and change-tracking counters
  are only a few microseconds at 100 rows.
- Large 10K rows still show multi-millisecond row building and commit/page
  pressure with `2006` page-pool misses, but recent page-builder and page-pool
  capacity candidates did not survive the insert matrix.
- The first-run 1000-row `medium_6col` gap was not stable enough to target.

## Negative-Ledger Fence

The current counter pattern maps directly to already-rejected standalone
families:

- Row construction: direct concat encoder, integer placeholder text cache,
  row-value pooling, param-one concat encoder, and prepared direct INSERT
  row-template executor.
- Fixed setup: exact benchmark PRAGMA execute fast path.
- Microbatch: larger statement microbatch renewal window.
- Large-row page pressure: global page-buffer recycle capacity increase,
  retained page-run widening, broad depth-2 right-edge page-builder admission,
  prebuilt empty-root direct INSERT leaf page-run, and prepared direct INSERT
  leaf-writer serialization fusion.

## Alien/Extreme Opportunity Matrix

| Candidate | Basis | Score | Decision |
| --- | --- | ---: | --- |
| Retrying row-template/concat fusion | `row_build_us` is visible | `1.3` | Reject: same family is fenced by repeated matrix losses. |
| Setup/PRAGMA fast path | 100-row fixed ceremony is visible | `0.8` | Reject: exact benchmark PRAGMA fast path already lost the focused insert gate. |
| Microbatch window increase | 10K schema-validation work is visible | `1.0` | Reject: standalone `max_r` increase is fenced until schema validation dominates. |
| True fused row/page builder | row-build plus page pressure are both visible | `3.0` | Research only for now: must remove row-template construction and page-run/page costs together, and must first win repeated focused INSERT before full quick. |
| LeanStore-style swizzled buffer/page access | graveyard match for page access overhead | `1.5` | Reject for this slice: current INSERT gap is not dominated by page lookup self-time. |

## Decision

No source patch was attempted.

The next viable source contract is not another standalone micro-optimization.
It needs to be a fused row/page builder that removes row construction and
page-run/page-image costs together, with a same-window A/B gate:

1. repeated focused INSERT improves weighted score, average, geomean, P90/P99,
   and the 100-row fixed-cost tails;
2. large 10K `large_10col` rows improve without worsening small rows;
3. full quick improves weighted score and does not regress DML or concurrent
   writer tails.
