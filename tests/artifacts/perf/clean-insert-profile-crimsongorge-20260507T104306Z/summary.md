# Clean INSERT profile - CrimsonGorge - 2026-05-07T10:43Z

## Scope

Clean worktree profile of committed `main` at
`b67563383d3a6e12cac6a7aa70b7c7f1a13660ba`, after the benchmark binary
metadata guard landed. The shared checkout still had peer-owned dirty work, so
the benchmark binary was built and run from:

`/data/tmp/frankensqlite-crimsongorge-clean-b675-20260507T103616Z`

## Commands

```bash
env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-clean-b675-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf

FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-crimsongorge-clean-b675-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/clean-insert-profile-crimsongorge-20260507T104306Z/report-insert.json \
  --no-html
```

Raw outputs:

- `commit.txt`
- `fingerprint.txt`
- `stdout.txt`
- `stderr.txt`
- `report-insert.json`

## Result

- Sections: `6`
- Scenarios: `25`
- FrankenSQLite faster / comparable / C SQLite faster: `15 / 4 / 6`
- Primary observed insert score: `0.895597602182437`
- Geomean ratio: `0.9052237906336388`
- Median ratio: `0.8914100578508632`
- p90 ratio: `1.1997614465016673`
- p99 ratio: `1.4463033797750329`

## Remaining C SQLite wins

| Row | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| Transaction strategy `10000 rows / batched (1000/txn)` | `3.226339 ms` | `4.666265 ms` | `1.4463x` |
| Single transaction `large_10col`, `100 rows` | `0.146324 ms` | `0.178184 ms` | `1.2177x` |
| Transaction strategy `100 rows / single txn` | `0.073778 ms` | `0.088516 ms` | `1.1998x` |
| Transaction strategy `100 rows / batched (100/txn)` | `0.076583 ms` | `0.091631 ms` | `1.1965x` |
| Record-size `large_10col`, `10000 rows` | `9.282476 ms` | `10.966468 ms` | `1.1814x` |
| Single transaction `medium_6col`, `100 rows` | `0.100428 ms` | `0.117250 ms` | `1.1675x` |

## Hotspot split

For `fs_insert_txn_batched_small_3col_10000`:

- `insert_us=8483.4`
- `row_build_ns=1674752`
- `cursor_setup_ns=422587`
- `btree_insert_ns=1460885`
- `btree_leaf_payload_appends=8934`
- `btree_quick_balance_hits=57`
- `btree_conservative_reloads=57`

For `fs_insert_record_size_large_10col_10000`:

- `insert_us=9926.5`
- `commit_us=4419.1`
- `row_build_ns=5125715`
- `btree_insert_ns=796859`
- `commit_roundtrip_ns=2112604`
- `page_pool_misses=2006`

## Interpretation

The remaining top insert gap is still the non-empty batched right-edge append
case. Standalone row-build/template/layout and pager/page-pool ideas are fenced
by existing negative-ledger entries. The highest-EV next lever is a true
non-empty page builder, or a fused record-body plus page-layout builder, that
keeps the payload-append kernel and avoids commit-time row-at-a-time full-cell
replay.

The dirty peer B-tree cursor guard was verified separately with:

```bash
env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target \
  CARGO_BUILD_JOBS=16 \
  cargo test -p fsqlite-btree \
  test_table_append_after_last_position_repeated_after_existing_rows_crosses_split \
  -- --nocapture
```

Result: `1 passed`.
