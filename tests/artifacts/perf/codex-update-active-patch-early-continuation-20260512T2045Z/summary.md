# Direct UPDATE Active Patch-Run Early Continuation Candidate

- Date: 2026-05-12
- Base commit: `23bd2895e8764c79b2cb073cdcf95560ee48d808`
- Candidate state: dirty source-only patch in `crates/fsqlite-core/src/connection.rs`
- Workload: `comprehensive-bench --quick --filter update`
- Result: rejected; source patch was unwound.

Focused proof passed:

```text
cargo test -p fsqlite-core direct_simple_update -- --nocapture
```

Candidate focused DML medians:

| Scenario | F ms | C ms | F/C | F CV% |
|---|---:|---:|---:|---:|
| 100 rows / update 10 rows | 0.010941 | 0.020258 | 0.540x | 32.85 |
| 100 rows / delete 5 rows | 0.027502 | 0.004007 | 6.863x | 53.58 |
| 1000 rows / update 100 rows | 0.032230 | 0.037630 | 0.856x | 23.49 |
| 1000 rows / delete 50 rows | 0.052268 | 0.028363 | 1.843x | 44.34 |
| 10000 rows / update 1000 rows | 0.310942 | 0.457887 | 0.679x | 12.99 |
| 10000 rows / delete 500 rows | 0.365675 | 0.190056 | 1.924x | 21.53 |

Compared with the current focused baseline
`tests/artifacts/perf/codex-current-dml-profile-a7635094-20260512T2005Z/update-delete-profile.json`,
the candidate worsened FSQLite UPDATE medians on all three update rows:
`0.006322ms -> 0.010941ms`, `0.029204ms -> 0.032230ms`, and
`0.254166ms -> 0.310942ms`. The patch is not a keep.
