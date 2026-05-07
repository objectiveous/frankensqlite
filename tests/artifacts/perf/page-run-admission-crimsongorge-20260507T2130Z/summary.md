# Direct INSERT page-run admission scratch A/B

Date: 2026-05-07
Agent: CrimsonGorge
Scratch worktree: `/data/tmp/frankensqlite-pagerun-admission-crimsongorge-20260507T2145Z`

## Purpose

The shared source files for direct INSERT were exclusively reserved by TanBear,
so this candidate was measured in a detached scratch worktree without touching
the main checkout. The idea came from the expected-loss admission rule for the
existing B-epsilon-style page-run buffer: current `connection.rs` admits
records at `16` bytes, which may over-buffer small records whose per-row
batching overhead exceeds avoided B-tree work.

## Candidate

Patch: `candidate-threshold128.diff`

- `PREPARED_DIRECT_INSERT_PAGE_RUN_MIN_RECORD_BYTES: 16 -> 128`
- `PREPARED_DIRECT_INSERT_PAGE_RUN_ARENA_MAX_RECORD_BYTES` unchanged at `384`

## Build and Benchmark

Build:

```bash
env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-pagerun-admission-target \
  CARGO_BUILD_JOBS=16 \
  rch exec -- cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

RCH could not normalize the `/data/tmp` scratch path and failed open to a local
build, which completed successfully.

Focused candidate run:

```bash
/data/tmp/frankensqlite-pagerun-admission-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/page-run-admission-crimsongorge-20260507T2130Z/candidate-insert-threshold128.json \
  --no-html
```

Comparator: `tests/artifacts/perf/current-write-profile-tanbear-20260507T2000Z/current-insert-profile.json`

## Result

Rejected. The focused INSERT gate moved the wrong way:

| Metric | Current | Threshold 128 |
| --- | ---: | ---: |
| primary score | `0.8190610418616213` | `0.843813033391191` |
| average ratio | `0.8473685692287133` | `1.0182896196063238` |
| geomean ratio | `0.8178930649378039` | `0.9698058208226569` |
| p90 ratio | `1.1504490521484194` | `1.3905761933793013` |
| p99 ratio | `1.2309899121331915` | `1.5402874678696952` |
| C SQLite faster rows | `8` | `11` |
| write_bulk geomean | `0.8176707820510124` | `0.9958567986163313` |
| write_single geomean | `0.8195249868923463` | `0.7984774849670893` |

The candidate did improve `write_single` geomean slightly, but it badly hurt
write-bulk and left the 100-row and 1000-row `small_3col` rows as the worst
INSERT gaps. This is not a keep.

## Decision

Do not raise the direct INSERT page-run admission floor to `128` as a
standalone change. Revisit admission only with a richer per-record or per-run
policy that uses row count, estimated leaf occupancy, and flush target shape,
and prove the full insert section before running the full quick matrix.
