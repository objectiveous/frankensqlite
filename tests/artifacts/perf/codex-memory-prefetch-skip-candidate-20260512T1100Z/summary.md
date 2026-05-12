# Private Memory Prefetch Skip Rejection

- Date: 2026-05-12
- Git baseline: `de579d54e9210670bdfe519dca064e0b8ebc1936`
- Candidate: early return from `SimpleTransaction::prefetch_page_hint()` for
  private `:memory:` transactions using `memory_db_bump_alloc`.
- Target: prefetch cost seen in sparse isolated DELETE profiling and remaining
  `UPDATE/DELETEThroughput` DELETE rows.

## Evidence

- Baseline artifact:
  `tests/artifacts/perf/codex-memory-prefetch-skip-baseline-20260512T1100Z/`
- Candidate artifact:
  `tests/artifacts/perf/codex-memory-prefetch-skip-candidate-20260512T1100Z/`
- Correctness smoke before benchmarking:
  `cargo test -p fsqlite-pager prefetch -- --nocapture`

## Result

Rejected and unwound. The focused update-filter geomean worsened from
`1.4012x` to `1.4881x` F/C, and average ratio worsened from `1.5790x` to
`1.8665x` F/C.

The decisive regression was `100 rows / delete 5 rows`: FSQLite moved from
`6.9us` (`3.01x` F/C) to `10.9us` (`4.81x` F/C). Larger DELETE rows were only
slightly better and within noise for this candidate (`29.8us` to `29.4us`,
`263.5us` to `260.1us`), so the lever does not clear the keep gate.
