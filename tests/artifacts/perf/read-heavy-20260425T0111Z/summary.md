# Read-heavy `mt-read-bench` matrix

Run ID: `read-heavy-20260425T0111Z`
Artifact root: `tests/artifacts/perf/read-heavy-20260425T0111Z/`
Bench binary: `crates/fsqlite-e2e/src/bin/mt_read_bench.rs`

## Source State

Current-HEAD matrix was run after rebuilding the bench binary.

- `HEAD`: `0e127332f956daf881ddbbf0c1e008c73496ef40`
- `HEAD` before current matrix: `current_head_before_matrix.txt`
- `HEAD` after current matrix: `current_head_after_matrix.txt`
- Worktree: dirty from other in-flight agents; exact state captured in `final_status.txt`
- Build log: `build.stderr`
- Environment fingerprint: `fingerprint.txt`

No code was changed for this artifact.

## Command

```bash
env CARGO_TARGET_DIR=/tmp/rch_target_navy_read \
  cargo build --release -p fsqlite-e2e --bin mt-read-bench

/tmp/rch_target_navy_read/release/mt-read-bench \
  --rows=5000 --reads-per-thread=20000 --threads=1,2,4,8
```

The matrix was run three times after the final rebuild:

- `matrix_current_head_run1.stdout` / `.stderr`
- `matrix_current_head_run2.stdout` / `.stderr`
- `matrix_current_head_run3.stdout` / `.stderr`

## Median Results

Medians from `matrix_current_head_run1..3`.

| Threads | FrankenSQLite rps | C SQLite rps | Ratio |
|---:|---:|---:|---:|
| 1 | 19 131 | 289 328 | 0.07x |
| 2 | 49 518 | 439 318 | 0.11x |
| 4 | 93 703 | 499 519 | 0.19x |
| 8 | 133 217 | 436 093 | 0.31x |

This confirms the read-heavy path is still well behind C SQLite even though the 8-thread write-heavy cumulative-verify result is favorable. The shape is consistent with the earlier `ad899fac` read-side matrix: FrankenSQLite scales with more reader threads, but the 1-thread base cost is too high.

## Profile Addendum

I also captured a bounded 1-thread profile for the doc-named next priority:

```bash
perf record -F 999 \
  --output perf_flat_1t_200k.data -- \
  /tmp/rch_target_navy_read/release/mt-read-bench \
  --rows=5000 --reads-per-thread=200000 --threads=1
```

Profile output:

- `profile_flat_1t_200k.stdout`
- `profile_flat_1t_200k.stderr`
- `perf_flat_1t_200k.data`
- `perf_flat_1t_200k_no_children.txt`
- `perf_flat_1t_200k_mt_read_bench_only.txt`
- `perf_flat_1t_200k_filtered.txt`

1-thread profile run result:

| Threads | FrankenSQLite rps | C SQLite rps | Ratio |
|---:|---:|---:|---:|
| 1 | 18 631 | 285 996 | 0.07x |

Top user-space symbols in the bench DSO:

| Rank | Symbol | Overhead |
|---:|---|---:|
| 1 | `PagerInner<IoUringFile>::refresh_committed_state` | 0.60% |
| 2 | `PagerBackend::begin` | 0.55% |
| 3 | `Cx::checkpoint` | 0.48% |
| 4 | `VdbeEngine::execute_with_borrowed_bindings_internal` | 0.39% |
| 5 | `UnixFile::lock` / `nix::fcntl::fcntl` / `posix_lock_with_timeout` cluster | 0.28-0.39% each |

Interpretation: the next profile-driven target should be the prepared-read autocommit/pager-begin path, especially per-read committed-state refresh, snapshot publication refresh, and file-lock/syscall overhead. The B-tree cell parse path is present but lower in this flat profile.

## Caveats

- The shared worktree was dirty throughout the final matrix; this artifact records the exact state instead of pretending it was a clean commit.
- Other local cargo/rustc jobs were active on the machine, so treat these as directionally useful and not release-grade isolated numbers.
- `perf` kernel symbol resolution was restricted by current kernel settings; no sudo/kernel tuning was applied.
- The first attempted `perf record --call-graph none` failed before running because this `perf` does not accept `none` for `--call-graph`; stderr is preserved in `profile_1t_200k.stderr`.
