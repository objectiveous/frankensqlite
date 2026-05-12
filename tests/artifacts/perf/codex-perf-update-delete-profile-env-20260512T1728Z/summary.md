# Focused UPDATE/DELETE Profile Env Probe

Date: 2026-05-12

Source head before this patch: `0ec06a6d4148b16881c8f614ac1e82b2efabbff0`

## Purpose

Verify that `perf-update-delete` now emits hot-path DML counters when
`FSQLITE_BENCH_PROFILE_DML=1` is set. This is measurement-only instrumentation
for the current DELETE tail; default benchmark execution is unchanged when the
environment variable is unset.

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  1000 1 delete compare standard \
  > tests/artifacts/perf/codex-perf-update-delete-profile-env-20260512T1728Z/stdout.txt \
  2> tests/artifacts/perf/codex-perf-update-delete-profile-env-20260512T1728Z/stderr.txt
```

RCH could not normalize the `/data/tmp` worktree path and fell back to local
execution.

## Result

The profile-enabled run produced the expected single `dml_profile` line for the
FrankenSQLite DELETE window:

```text
[fsqlite standard delete iter=0 rows=1000] dml_profile elapsed_us=69.8 direct_update=0 direct_delete=50 delete_leaf_start=6/6 delete_leaf_active=44/49 delete_leaf_miss=5 delete_leaf_flush=6/6 delete_leaf_materialize=6/6502 delete_leaf_write=6/1253 fast=50 slow=0
```

The same run reported the expected focused comparison line:

```text
fsqlite/sqlite time ratio: total=2.85x populate=1.08x update=0.00x delete=3.56x
```

This artifact proves the focused binary can now capture the DELETE leaf-run,
pager, parser, and record counters directly, without needing to rerun the full
`comprehensive-bench --quick --filter update` harness for every local profile
probe.

A no-env smoke run of `perf-update-delete 100 1 delete fsqlite standard`
emitted no `dml_profile` line, confirming the default path remains quiet.
