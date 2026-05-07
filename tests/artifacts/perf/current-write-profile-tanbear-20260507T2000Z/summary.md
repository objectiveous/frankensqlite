# Current write-path profile

Date: 2026-05-07
Agent: TanBear
Source head at capture: `506711e22f22bf749e932771396452fb6c88952d`

## Purpose

This bundle publishes the measurement pass that led to the retained direct-DML
cursor handoff. It covers the current INSERT and UPDATE/DELETE sections after
the direct INSERT layout keep gate, plus a user-space `perf` sample for the
INSERT section.

The release-perf benchmark binary was built in
`/data/tmp/frankensqlite-current-write-profile-target/release-perf`.

## INSERT Section

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-current-write-profile-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/current-write-profile-tanbear-20260507T2000Z/current-insert-profile.json \
  --no-html
```

Summary from `current-insert-profile.json`:

- Scenarios: `25`
- FrankenSQLite faster / comparable / C SQLite faster: `17 / 0 / 8`
- Average ratio: `0.8473685692287133`
- Geomean ratio: `0.8178930649378039`
- p90 / p99 ratio: `1.1504490521484194 / 1.2309899121331915`
- write_bulk geomean: `0.8176707820510124`
- write_single geomean: `0.8195249868923463`

Top C-faster rows:

| Ratio | Scenario |
| ---: | --- |
| `1.2309899121331915` | `INSERT Single Transaction medium_6col 1000 rows` |
| `1.155926999377986` | `INSERT Single Transaction small_3col 100 rows` |
| `1.1504490521484194` | `INSERT Transaction Strategy small_3col 100 rows / batched` |
| `1.1340756631593285` | `INSERT Single Transaction large_10col 100 rows` |
| `1.1285529715762275` | `INSERT Single Transaction tiny_1col 100 rows` |

Hot profile counters:

- `fs_insert_single_txn_medium_6col_1000`: `execute_body_ns=716879`,
  `row_build_ns=177931`, `btree_insert_ns=94473`,
  `commit_roundtrip_ns=115136`.
- `fs_insert_record_size_large_10col_10000`: `execute_body_ns=11326267`,
  `row_build_ns=4073284`, `btree_insert_ns=813180`,
  `commit_roundtrip_ns=2419672`.
- `fs_insert_single_txn_small_3col_100`: `row_build_ns=11383`,
  `btree_insert_ns=3261`.

Interpretation: INSERT is now mostly a row-builder/record-layout problem at
small and large row shapes. The broad `perf` sample confirms the top
FrankenSQLite self symbol is
`Connection::try_serialize_prepared_direct_simple_insert_record` at `7.54%`.
Standalone concat, varint, and generic record-layout helpers are fenced in the
negative ledger; any next INSERT source slice should be a fused direct record
builder plus page-run writer, not another local helper patch.

## UPDATE/DELETE Section

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-current-write-profile-target/release-perf/comprehensive-bench \
  --quick \
  --filter update \
  --json-out tests/artifacts/perf/current-write-profile-tanbear-20260507T2000Z/current-update-profile.json \
  --no-html
```

Summary from `current-update-profile.json`:

- Scenarios: `6`
- FrankenSQLite faster / comparable / C SQLite faster: `2 / 1 / 3`
- Average ratio: `1.0739937319061605`
- Geomean ratio: `1.0582038086424348`
- p90 / p99 ratio: `1.3474869311103608 / 1.3474869311103608`

Rows:

| Ratio | Scenario |
| ---: | --- |
| `1.3474869311103608` | `100 rows / delete 5 rows` |
| `1.3046705304660235` | `100 rows / update 10 rows` |
| `1.0536656233384472` | `10000 rows / update 1000 rows` |
| `0.9687597374915649` | `10000 rows / delete 500 rows` |
| `0.8987948462908562` | `1000 rows / update 100 rows` |
| `0.8705847227397111` | `1000 rows / delete 50 rows` |

Hot profile counters:

- `fs_update_100`: `setup_us=53.3`, `prepare_us=24.1`,
  `mutate_us=12.4`, `commit_us=6.1`, `direct_update=10`, `fast=10`,
  `slow=0`.
- `fs_delete_100`: `setup_us=54.0`, `prepare_us=17.1`,
  `mutate_us=8.6`, `commit_us=5.6`, `direct_delete=5`, `fast=5`,
  `slow=0`.
- `fs_update_10000`: `mutate_us=1259.6`, `commit_us=193.0`.
- `fs_delete_10000`: `mutate_us=678.2`, `commit_us=167.5`.

Interpretation: the small UPDATE/DELETE rows are real C-faster rows, but the
direct DML counters already bypass VDBE (`fast=count`, `slow=0`). Nearby local
ceremony tweaks are rejected in the ledger. The next allowed source candidate
is a retained direct-DML cursor kernel that removes per-row cursor construction
and root-to-leaf work.

## Perf Samples

Valid INSERT sample:

```bash
perf record -g --call-graph dwarf,8192 -F 999 \
  -- /data/tmp/frankensqlite-current-write-profile-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html
```

Text reports committed:

- `perf-insert/top.txt`
- `perf-insert/top-children.txt`
- `perf-insert/bench.log`

Top user-space symbols by self overhead in `perf-insert/top.txt`:

| Self | Symbol |
| ---: | --- |
| `14.25%` | `sqlite3VdbeExec` |
| `7.62%` | `__memmove_avx_unaligned_erms` |
| `7.54%` | `Connection::try_serialize_prepared_direct_simple_insert_record` |
| `3.03%` | `__memset_avx2_unaligned_erms` |
| `2.63%` | `sqlite3BtreeTableMoveto` |
| `1.89%` | `Connection::execute_prepared_direct_simple_insert` |
| `1.65%` | `Connection::eval_prepared_direct_simple_insert_expr` |

Invalid sample: `perf-medium6` used `--filter medium_6col`, which matched no
sections in `comprehensive-bench`. It mostly sampled startup/toolchain noise and
is preserved only as a caution. Do not use it for keep/reject decisions.

Raw `perf.data` files were not added to Git; the committed text reports are the
durable evidence surface.

## Decision

The current write profile supports two priorities:

1. Retained direct-DML cursor kernel for UPDATE/DELETE once
   `connection.rs` / `cursor.rs` are unreserved.
2. Fused direct INSERT record/page-run builder later, if the retained DML lane
   is blocked or completed.

Do not retry standalone scratch reset, varint, concat-expression, or generic
record serializer changes without a new profile that makes one of those helpers
a dominant top-level self-time symbol.
