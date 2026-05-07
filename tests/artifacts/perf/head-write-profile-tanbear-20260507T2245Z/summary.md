# Fresh HEAD write-path profile

Captured from clean `main` at `19904b5e2daf5ebd06d6da9f9da12c8c82d4c851`
(`git_dirty: false`) with the `release-perf` benchmark binary built by RCH.

## Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-head-profile-target CARGO_BUILD_JOBS=10 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-head-profile-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/head-write-profile-tanbear-20260507T2245Z/head-insert-profile.json \
  --no-html

FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-head-profile-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/head-write-profile-tanbear-20260507T2245Z/head-update-profile.json \
  --no-html
```

Raw stdout/stderr are under `stdout/`.

## INSERT profile

Summary:

- Scenarios: 25
- Franken faster / comparable / C faster: 15 / 2 / 8
- Average / geomean / median ratio: `0.860789` / `0.827512` / `0.817804`
- p90 / p99 ratio: `1.186809` / `1.343796`
- Weighted score: `0.857832`

Largest remaining C-faster rows:

| Ratio | Section | Scenario | F ms | C ms |
| ---: | --- | --- | ---: | ---: |
| `1.343796` | Single Transaction small_3col | 100 rows | `0.104636` | `0.077866` |
| `1.225083` | Single Transaction medium_6col | 1000 rows | `0.701473` | `0.572592` |
| `1.186809` | Record Size Comparison | large_10col 10K | `11.133023` | `9.380633` |
| `1.122589` | Transaction Strategy small_3col | 100 rows / autocommit | `0.139091` | `0.123902` |
| `1.101213` | Single Transaction medium_6col | 10000 rows | `6.729589` | `6.111072` |

Profile read:

- Small 100-row INSERT gaps are fixed-cost dominated (`setup+begin+prepare+commit`
  are a large fraction of the row).
- Large `large_10col` 10K remains the most material INSERT row. It shows
  `row_build_ns=4071365`, `btree_insert_ns=723286`,
  `commit_roundtrip_ns=2221659`, and `page_pool_misses=2006`.
- The surviving large-row gap is not a standalone varint/header micro-path;
  recent record/header and param-one probes already fenced those shapes in the
  negative ledger.

## UPDATE/DELETE profile

Summary:

- Scenarios: 6
- Franken faster / comparable / C faster: 1 / 3 / 2
- Average / geomean / median ratio: `1.187422` / `1.137121` / `1.031949`
- p90 / p99 ratio: `2.058668` / `2.058668`
- Weighted score: `1.137121`

Rows:

| Ratio | Scenario | F ms | C ms |
| ---: | --- | ---: | ---: |
| `2.058668` | 100 rows / update 10 rows | `0.182783` | `0.088787` |
| `1.160419` | 100 rows / delete 5 rows | `0.116679` | `0.100549` |
| `1.031949` | 10000 rows / update 1000 rows | `3.938885` | `3.816936` |
| `0.990848` | 1000 rows / update 100 rows | `0.418674` | `0.422541` |
| `0.973447` | 10000 rows / delete 500 rows | `3.517145` | `3.613085` |
| `0.909201` | 1000 rows / delete 50 rows | `0.353532` | `0.388838` |

Profile read:

- The worst row is small-update setup/prepare dominated:
  `fs_update_100` has `setup_us=70.6`, `prepare_us=34.9`,
  `mutate_us=20.5`, and `commit_us=10.3`.
- Direct UPDATE/DELETE mutation itself is already VDBE-bypassed
  (`vdbe_opcodes=0`, direct counts equal mutations).
- Same-leaf fixed-width UPDATE batching and leaf-hint variants were rejected by
  isolated gates; do not retry them unless the design becomes a true retained
  leaf-run operator that also removes per-row admission/projection/mirror cost.

## Next optimization target

The best next measured lever is not another local row helper. The profile points
to a larger fused direct-INSERT row/page-run builder for large records, with a
strict keep gate on:

1. `large_10col` 10K record-size row,
2. `medium_6col` 1K and 10K rows,
3. the small 100-row p99 rows, so the builder does not regress fixed-cost cases.

For DML, the next useful work is a fresh design around setup/prepare amortization
or a real retained cursor/leaf-run kernel. Standalone leaf hints, same-leaf batch
flushes, and borrowed-context probes are already rejected.
