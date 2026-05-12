# Current Concurrent Profile After Shared-Table Retry Fix

- Date: 2026-05-12 12:40 UTC
- Commit: `7ce06a0e72695eb1696c58efd4cc6c009a6ca8b3`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-concurrent-7ce06a0e CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_CONCURRENT=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter concurrent --no-html --json-out tests/artifacts/perf/codex-concurrent-current-profile-after-mtfix-20260512T1250Z/concurrent-current.json`
- Validity: `git_dirty=false`, `benchmark_binary_older_than_git_head=false`.

## Ratios

This is a profiled run, so use it primarily for counter attribution rather than as a replacement for the unprofiled full-quick frontier.

| F/C | Scenario |
| ---: | --- |
| `1.1850248764511535` | `2 writers x 1000 rows` |
| `1.2237676738824395` | `4 writers x 1000 rows` |
| `0.6810617027202382` | `8 writers x 1000 rows` |

## Counters

All profiled rows stayed on prepared direct INSERT (`slow=0`) with no page-run flushes.

| Writers | Direct inserts | Commit attempts/success/errors | Page-lock waits | Stale snapshots | Lock wait ns | Candidate-free fast paths | Full validations |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| `2` | `24012` | `36/36/0` | `12` | `12` | `18265716` | `0` | `12` |
| `4` | `58178` | `70/60/10` | `85` | `72` | `157288723` | `0` | `36` |
| `8` | `144170` | `153/108/45` | `493` | `311` | `1054590906` | `0` | `76` |

## Decision

No source patch from this profile. The low-thread rows still point at the already-fenced concurrent boundary: page-lock waits and retryable stale-snapshot churn are part of the current SSI/FCW publication shape, while `candidate_free_fast_paths=0` remains intentional because safe commit planning depends on hydrated read/write witnesses and active/committed candidate sets. The 8-writer row stays faster despite the same churn, so standalone wait-slice, retry/backoff, witness-container, or page-run admission tweaks are not justified without a broader MVCC publication design.
