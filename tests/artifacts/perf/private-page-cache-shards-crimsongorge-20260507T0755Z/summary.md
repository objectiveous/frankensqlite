# Private `:memory:` Page-Cache Shard Candidate

Read-only measurement of PurpleOtter's reserved dirty-tree candidate on
2026-05-07. I did not edit, stage, revert, or commit the owned pager files.

## Candidate Shape

- `crates/fsqlite-pager/src/page_cache.rs`: add
  `ShardedPageCache::with_max_buffers_for_initial_pages_single_connection`,
  using `MIN_PAGE_CACHE_SHARDS` for the fallback shard tier.
- `crates/fsqlite-pager/src/pager.rs`: route private `/:memory:` pager opens to
  that constructor.
- Full dirty diff captured in `candidate.diff`.

## Correctness / Build Checks

- `cargo fmt -p fsqlite-pager --check` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-private-page-cache-target cargo test -p fsqlite-pager test_single_connection_initial_page_hint_keeps_fallback_shards_small -- --nocapture` passed.
- Candidate release-perf build passed:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-private-page-cache-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`.
- Baseline release-perf build from detached clean worktree `b11dde79` passed after
  one transient missing-rustc retry:
  `/data/tmp/frankensqlite-crimsongorge-baseline-b11dde79-20260507T0800`.

## Focused Transaction Section

Same-window `--quick --filter transaction` was mixed.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 1.229261 | 1.233645 |
| Geomean ratio | 1.139352 | 1.130338 |
| Median ratio | 1.196162 | 1.154076 |
| Write-bulk geomean | 1.072184 | 1.053957 |
| Write-single geomean | 1.286578 | 1.300107 |

Absolute FrankenSQLite medians improved in most transaction rows, but
`1000 rows / autocommit` and `10000 rows / autocommit` regressed.

## Full Quick Matrix

The full quick matrix is a likely keep by the primary project gate.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 0.374632 | 0.371643 |
| Average ratio | 0.499617 | 0.489023 |
| Geomean ratio | 0.276333 | 0.277578 |
| Median ratio | 0.301447 | 0.291390 |
| P90 ratio | 1.203237 | 1.121600 |
| P99 ratio | 1.585629 | 1.496608 |
| FrankenSQLite faster rows | 72 | 74 |
| Comparable rows | 5 | 5 |
| C SQLite faster rows | 16 | 14 |
| Write-bulk geomean | 0.855160 | 0.854591 |
| Write-single geomean | 1.226553 | 1.147031 |

Disposition: read-only evidence for the reservation holder. I recommend
landing the candidate only with the holder's own final check/commit, because
they own the pager edit reservation.

## Follow-up

PurpleOtter later reported repeated focused UPDATE/DELETE regressions for the
same candidate family and reverted their source diff before commit. The negative
ledger records this as an abandoned candidate with conflicting evidence rather
than a clean full-matrix rejection.
