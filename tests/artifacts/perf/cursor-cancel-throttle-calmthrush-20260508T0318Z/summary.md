# Cursor Cancellation Throttle Probe

Date: 2026-05-08 03:18Z
Agent: CalmThrush
Head: `6726103a docs(perf): map direct insert row template`

## Candidate

In `crates/fsqlite-btree/src/cursor.rs`, temporarily changed the expensive
cursor checkpoint throttle interval in `observe_cursor_cancellation()`:

```diff
- const THROTTLE_INTERVAL: u32 = 64;
+ const THROTTLE_INTERVAL: u32 = 256;
```

The cheap `cx.is_cancel_requested()` atomic check still ran on every cursor
operation. The source change was reverted after measurement.

## Build

Baseline and candidate used the same local target directory. Baseline was run
before the source edit; candidate was rebuilt after the one-line edit.

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cursor-throttle-baseline-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e \
  --bin comprehensive-bench --bin perf-update-delete
```

Candidate build also passed:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cursor-throttle-baseline-target \
  CARGO_BUILD_JOBS=16 \
  cargo fmt -p fsqlite-btree --check

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cursor-throttle-baseline-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e \
  --bin comprehensive-bench --bin perf-update-delete
```

## Gate

Focused gate:

```text
comprehensive-bench --quick --filter update --no-html
```

Artifacts:

- `baseline-update.json`
- `candidate-update-throttle256.json`
- raw stdout/stderr under `stdout/`

## Result

Rejected and reverted. The focused UPDATE/DELETE section worsened:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| average ratio | `1.0966731080555328` | `1.1182863386817876` |
| geomean ratio / score | `1.0747695002967321` | `1.1076584444593147` |
| p90 ratio | `1.4835788635458733` | `1.4087949335474088` |
| p99 ratio | `1.4835788635458733` | `1.4087949335474088` |
| C SQLite faster rows | `2` | `4` |

Mixed row movement:

| Row | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| `100 rows / update 10 rows` | `1.483579` | `1.244795` | `0.130414` | `0.118502` |
| `100 rows / delete 5 rows` | `1.344861` | `1.408795` | `0.109395` | `0.110558` |
| `1000 rows / update 100 rows` | `0.944483` | `0.985875` | `0.394078` | `0.388769` |
| `1000 rows / delete 50 rows` | `0.905676` | `0.955305` | `0.364012` | `0.349054` |
| `10000 rows / update 1000 rows` | `0.978398` | `1.051520` | `3.625225` | `3.731353` |
| `10000 rows / delete 500 rows` | `0.923042` | `1.063428` | `3.184509` | `3.508646` |

Do not retry increasing the cursor cancellation checkpoint interval as a
standalone btree optimization. Reconsider only if a fresh profile names
`observe_cursor_cancellation()` or `Cx::checkpoint()` as retained self-time and
the same-window focused section improves, including the 10K rows.
