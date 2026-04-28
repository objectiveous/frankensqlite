# WAL checksum transform candidate

Date: 2026-04-28
Agent: IcyBluff

## Candidate

`8ee02146 refactor(wal/checksum): chain frame checksum via then_aligned_bytes`

The candidate changed `WalChecksumTransform::for_wal_frame` to stream the
8-byte frame header and page payload through one transform, avoiding the
separate header/payload transform construction and final `then` composition.

## Verdict

Rejected. The existing WAL scratch benchmark did not show a meaningful win.

## Benchmark

Baseline build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-baseline \
  cargo test -p fsqlite-wal --profile release-perf wal_frame_scratch_benchmark_report --no-run
```

Candidate build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-candidate \
  cargo test -p fsqlite-wal --profile release-perf wal_frame_scratch_benchmark_report --no-run
```

Measured command:

```bash
hyperfine --warmup 1 --runs 10 \
  --export-json tests/artifacts/perf/20260428T0900Z-icybluff-wal-checksum/hyperfine-wal-scratch.json \
  --command-name baseline-wal-scratch '/data/tmp/cargo-target-icybluff-20260428-wal-baseline/release-perf/deps/fsqlite_wal-bba5890cb2f6611c --ignored --exact wal::tests::wal_frame_scratch_benchmark_report --nocapture' \
  --command-name candidate-wal-scratch '/data/tmp/cargo-target-icybluff-20260428-wal-candidate/release-perf/deps/fsqlite_wal-bba5890cb2f6611c --ignored --exact wal::tests::wal_frame_scratch_benchmark_report --nocapture'
```

| Scenario | Baseline mean | Candidate mean | Delta |
| --- | ---: | ---: | ---: |
| `wal_frame_scratch_benchmark_report` | 331.209ms | 329.915ms | 0.39% faster |

The delta is inside run noise (`baseline sigma=5.161ms`, `candidate
sigma=4.179ms`), so the candidate does not meet the profile-driven bar for a
hot-path change.

## Follow-up

The rollback restores the previous `from_aligned_bytes` plus `header.then(payload)`
shape in `for_wal_frame`.

Verification after rollback:

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-verify \
  cargo test -p fsqlite-wal test_wal_checksum_transform_matches_frame_checksum -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-verify \
  cargo check -p fsqlite-wal --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-verify \
  cargo clippy -p fsqlite-wal --all-targets -- -D warnings
ubs crates/fsqlite-wal/src/checksum.rs
```
