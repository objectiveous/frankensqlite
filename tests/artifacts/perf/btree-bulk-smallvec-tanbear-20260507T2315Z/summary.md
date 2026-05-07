# B-tree Bulk SmallVec Probe

Date: 2026-05-07
Agent: TanBear
Base source: `f3605bef docs(perf): profile fresh head write gaps`
Candidate source: clean detached worktree with only `crates/fsqlite-btree/src/cursor.rs` patched.

## Candidate

Replace two transient `Vec<u16>` cell-pointer buffers in the bulk table leaf and
interior page builders with `SmallVec<[u16; 256]>`. This preserves the existing
`cell::write_cell_pointers` path and is intentionally not the previously rejected
direct-pointer-write probe.

## Measurement

Command shape:

```text
FSQLITE_BENCH_PROFILE_INSERT=1 comprehensive-bench --quick --filter insert --json-out <report> --no-html
```

The first candidate report, `candidate-insert-profile.json`, was built from the
shared dirty worktree and is therefore informational only. The retained evidence
is the clean isolated candidate worktree plus same-window baseline/candidate
samples.

| Sample | Weighted score | Avg ratio | Geomean ratio | P90 | P99 | large_10col record F ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Fresh HEAD profile | `0.857832` | `0.860789` | `0.827512` | `1.186809` | `1.343796` | `11.133023` |
| Same-window baseline 1 | `0.826753` | `0.930832` | `0.885283` | `1.272607` | `2.036567` | `19.594114` |
| Isolated candidate 1 | `0.810192` | `0.843089` | `0.812560` | `1.133197` | `1.422925` | `10.247711` |
| Same-window baseline 2 | `0.798231` | `0.866625` | `0.838073` | `1.191930` | `1.256491` | `9.340743` |
| Isolated candidate 2 | `0.751712` | `0.811720` | `0.788758` | `1.082498` | `1.151025` | `9.803429` |

Interpretation: keep. Baseline 1 had a clear large-record outlier, so the
high-signal comparison is baseline 2 versus candidate 2, where the weighted
score improved from `0.798231` to `0.751712`, average ratio improved from
`0.866625` to `0.811720`, geomean improved from `0.838073` to `0.788758`, and
tail ratios improved. The candidate also improved versus the earlier committed
fresh HEAD profile.

## Verification

Passed:

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-btree-smallvec-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-btree table_bulk -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-btree-smallvec-target CARGO_BUILD_JOBS=8 cargo fmt -p fsqlite-btree --check`
- `git diff --check -- crates/fsqlite-btree/src/cursor.rs tests/artifacts/perf/btree-bulk-smallvec-tanbear-20260507T2315Z`
- `ubs crates/fsqlite-btree/src/cursor.rs`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-btree-smallvec-target CARGO_BUILD_JOBS=8 cargo check -p fsqlite-btree --all-targets`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-btree-smallvec-target CARGO_BUILD_JOBS=8 cargo clippy -p fsqlite-btree --all-targets -- -D warnings`
