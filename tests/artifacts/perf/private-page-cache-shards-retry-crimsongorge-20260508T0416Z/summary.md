# Private Memory Page-Cache Fallback Shard Retry

Date: 2026-05-08

Candidate source was applied in the shared checkout, built into
`/data/tmp/frankensqlite-private-shards-retry-target/release-perf/comprehensive-bench`,
then reverted after the focused UPDATE/DELETE gate rejected it.

## Candidate

- `crates/fsqlite-pager/src/page_cache.rs`: add
  `ShardedPageCache::with_max_buffers_for_initial_pages_single_connection`,
  using `MIN_PAGE_CACHE_SHARDS` for the overflow/fallback shard tier.
- `crates/fsqlite-pager/src/pager.rs`: route private `/:memory:` pager opens to
  that constructor.
- Rationale: private `:memory:` connections immediately enable the flat-array
  fast path, so the 128-shard overflow tier is usually cold open/setup cost.

## Correctness / Build

- `cargo fmt -p fsqlite-pager --check` passed.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-private-shards-retry-test-target CARGO_BUILD_JOBS=12 cargo test -p fsqlite-pager test_single_connection_initial_page_hint_keeps_fallback_shards_small -- --nocapture` passed; the RCH artifact retrieval was terminated locally after the green test result.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-private-shards-retry-target CARGO_BUILD_JOBS=16 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench` passed.

## Focused UPDATE/DELETE Gate

Baseline: `tests/artifacts/perf/current-post-dml-tanbear-20260508T0110Z/update-profile-current.json`.

Candidate:
`tests/artifacts/perf/private-page-cache-shards-retry-crimsongorge-20260508T0416Z/candidate-update.json`.

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.325330 | 1.280169 | 0.119224 | 0.114995 |
| 100 rows / delete 5 rows | 1.459979 | 1.397376 | 0.113874 | 0.110137 |
| 1000 rows / update 100 rows | 0.997968 | 1.024490 | 0.403266 | 0.405659 |
| 1000 rows / delete 50 rows | 1.039226 | 1.033894 | 0.372148 | 0.374682 |
| 10000 rows / update 1000 rows | 1.050288 | 1.136525 | 3.867106 | 4.072737 |
| 10000 rows / delete 500 rows | 1.074879 | 1.224005 | 3.475211 | 4.129804 |

Focused section average/geomean worsened from `1.157945 / 1.146025` to
`1.182743 / 1.175315`. C-SQLite-faster rows stayed at `4`.

## Disposition

Rejected and reverted. The retry reproduced the earlier split evidence: tiny
open/setup-heavy rows improved, but larger UPDATE/DELETE rows regressed enough
to fail the section gate. I stopped before a full quick matrix because the
ledger retry condition required the focused gate and matrix to both move in the
right direction.

