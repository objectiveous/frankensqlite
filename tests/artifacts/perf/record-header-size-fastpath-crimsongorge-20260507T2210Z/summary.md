# Record Header Size Fast Path

Run ID: `record-header-size-fastpath-crimsongorge-20260507T2210Z`
Source baseline: `741d135e`
Candidate surface: `crates/fsqlite-types/src/record.rs`
Workload: `comprehensive-bench --quick --filter insert`

## Candidate

`compute_header_size(content_size)` can return immediately for
`content_size <= 126`. In that range, the final header size is
`content_size + 1`, which is at most `127`, so the header-size varint is
exactly one byte and the iterative fixed-point computation is unnecessary.

This is one lever only. It does not change record layout, serial-type
selection, value encoding, row ordering, transaction behavior, or MVCC state.

## Measurements

Lower ratios and weighted scores are better because they are
FrankenSQLite-over-C-SQLite ratios.

| Run | Franken faster | Comparable | C SQLite faster | Avg ratio | Geomean | P90 | P99 | Weighted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `baseline-insert.json` | 17 | 2 | 6 | 0.829544 | 0.807436 | 1.102348 | 1.117254 | 0.811157 |
| `baseline-insert-repeat.json` | 17 | 1 | 7 | 0.855784 | 0.820180 | 1.183566 | 1.599686 | 0.809937 |
| `baseline-insert-repeat2.json` | 20 | 1 | 4 | 0.770238 | 0.743933 | 1.088150 | 1.162441 | 0.765266 |
| `candidate-insert.json` | 20 | 2 | 3 | 0.763510 | 0.740902 | 1.055355 | 1.101204 | 0.763023 |
| `candidate-insert-repeat.json` | 16 | 5 | 4 | 0.794393 | 0.766839 | 1.067373 | 1.138116 | 0.766244 |
| `candidate-insert-repeat2.json` | 18 | 2 | 5 | 0.805628 | 0.778922 | 1.099838 | 1.155846 | 0.778348 |

Across the three samples, the weighted-score median moved from `0.809937` to
`0.766244` and the weighted-score mean moved from about `0.795454` to
`0.769205`. Individual rows were noisy and not a sweep, but the aggregate insert
matrix moved in the intended direction on both median and mean.

## Correctness

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-record-header-fastpath-candidate-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-types record -- --nocapture`
  passed in the detached candidate worktree.
- `cargo fmt --check` passed in the detached candidate worktree.

## Isomorphism Proof

- Ordering preserved: yes. The function only computes the serialized record
  header byte count.
- Tie-breaking unchanged: not applicable.
- Floating-point behavior: unchanged. Value encoding is untouched.
- RNG seeds: not applicable.
- Header bytes: unchanged. For `content_size <= 126`, the original loop starts
  with `header_size = content_size + 1`, computes `varint_len(header_size) == 1`,
  and returns the same value on the first iteration.
- Fallback: all `content_size > 126` values retain the existing fixed-point loop.
