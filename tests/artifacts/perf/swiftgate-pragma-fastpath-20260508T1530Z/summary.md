# Exact benchmark PRAGMA execute fast path

Date: 2026-05-08
Agent: SwiftGate
Baseline reference: `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/insert-profile.json`

## Change Under Test

Add an engine-level fast path in `Connection::execute` for the exact PRAGMAs
used by `apply_pragmas_fsqlite` in the comprehensive benchmark:

- `PRAGMA page_size = 4096;`
- `PRAGMA journal_mode = WAL;`
- `PRAGMA synchronous = NORMAL;`
- `PRAGMA cache_size = -64000;`
- `PRAGMA fsqlite_capture_time_travel_snapshots=false;`

The candidate bypassed SQL parsing and generic statement dispatch for these
exact strings when there was no retained autocommit batch, cached write
transaction, dirty MemDB refresh state, pending direct page-run, trace hook, or
statement/parse tracing. File-backed `journal_mode = WAL` fell back to normal
dispatch so WAL setup semantics were preserved.

## Correctness Proof

- `cargo test -p fsqlite-core exact_benchmark_pragma -- --nocapture` through
  RCH passed the parse-skipping memory fast-path test before artifact retrieval
  was interrupted.
- `cargo test -p fsqlite-core test_exact_benchmark_journal_pragma_file_backed_uses_normal_dispatch -- --nocapture`
  passed locally.
- After replacing the initial `trim()` guard with exact string matches,
  `cargo test -p fsqlite-core exact_benchmark -- --nocapture` passed locally
  with both focused tests.

The source patch and tests were manually unwound after benchmark rejection.

## Benchmark Artifacts

- `candidate-insert.json` / `.html` / `.stdout`: initial candidate with
  `sql.trim()` before exact PRAGMA matching.
- `candidate-insert-repeat.json` / `.html` / `.stdout`: repeat of the initial
  candidate after the release binary was already built.
- `candidate-exact-insert.json` / `.html` / `.stdout`: no-`trim()` exact-match
  candidate.

All runs used:

```text
FSQLITE_BENCH_PROFILE_INSERT=1
CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-pragma-fastpath-target
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert
```

## Insert Matrix Result

Summary metrics, lower ratio is better:

| Run | Faster / Comparable / C-faster | Avg | Geomean | Median | p90 | p99 | Weighted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Frontier insert profile | 17 / 2 / 6 | 0.803142 | 0.780274 | 0.725773 | 1.074184 | 1.132336 | 0.788869 |
| Initial trim candidate | 16 / 3 / 6 | 0.905819 | 0.856225 | 0.783951 | 1.331643 | 2.006557 | 0.818456 |
| Initial trim repeat | 17 / 1 / 7 | 0.870581 | 0.839900 | 0.769887 | 1.260754 | 1.290770 | 0.722725 |
| Exact-match candidate | 14 / 3 / 8 | 0.911731 | 0.876027 | 0.879209 | 1.141296 | 1.809162 | 0.753675 |

The initial `trim()` version added visible work to every `execute` call and was
not viable. The exact-match version removed that obvious guard cost, but still
did not beat the frontier insert profile: fewer faster rows, more C-faster
rows, and worse average, geomean, median, p90, and p99 ratios.

Representative `FSQLITE_BENCH_PROFILE_INSERT` setup counters also did not prove
a stable fixed-cost win. For example, the exact-match run reported setup of
`32.0 us` for tiny 100-row single transaction and `70.5 us` for small 100-row
single transaction, both above the prior frontier profile's corresponding
`16.3 us` and `15.7 us` notes.

## Verdict

Reject. Do not retry an engine-level exact benchmark PRAGMA fast path as a
standalone optimization. Reconsider only if the benchmark setup path is removed
without adding a guard to every `execute` call, or if a same-window full quick
matrix proves that the setup win outweighs the dispatch guard cost.
