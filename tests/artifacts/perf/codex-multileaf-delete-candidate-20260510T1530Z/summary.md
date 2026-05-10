# Multi-Leaf DELETE Buffer Candidate Rejection - 2026-05-10

## Scope

Screened a source candidate in `crates/fsqlite-core/src/connection.rs` that
kept prior same-table prepared direct DELETE leaf runs in a transaction-local
backlog after crossing leaf boundaries. The candidate also added a cursor
fallback guard so ordinary cursor deletes would flush any retained older leaf
image before mutating the B-tree.

The candidate passed the focused correctness tests during the experiment, then
was reverted because the target benchmark rows did not improve.

## Commands

Focused correctness run on the candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-multileaf-delete-target CARGO_BUILD_JOBS=4 cargo test -p fsqlite-core prepared_direct_delete_leaf_run -- --nocapture --test-threads=1
```

Candidate benchmark binary build and first remote screen:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-multileaf-delete-bench-target CARGO_BUILD_JOBS=4 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-multileaf-delete-candidate-20260510T1530Z/candidate-update-delete-quick.json --no-html
```

RCH did not retrieve the remote JSON file, so the retrieved candidate
`release-perf` binary was run directly to materialize the tracked JSON:

```bash
/data/tmp/frankensqlite-multileaf-delete-bench-target/release-perf/comprehensive-bench --quick --filter update-delete --json-out tests/artifacts/perf/codex-multileaf-delete-candidate-20260510T1530Z/candidate-update-delete-quick.json --no-html
```

## Evidence

Primary candidate artifact:

- `tests/artifacts/perf/codex-multileaf-delete-candidate-20260510T1530Z/candidate-update-delete-quick.json`

Comparison baseline:

- `tests/artifacts/perf/codex-fresh-frontier-full-quick-20260510T093306Z/full-quick.json`

| Scenario | Baseline ratio | Candidate ratio | Verdict |
| --- | ---: | ---: | --- |
| 100 rows / delete 5 rows | 3.187x | 3.427x | worse |
| 1000 rows / delete 50 rows | 2.007x | 2.032x | within noise / no useful win |
| 10000 rows / delete 500 rows | 1.945x | 2.332x | worse |

The candidate improved the two UPDATE rows in this isolated quick run, but that
was not the targeted change and came from a dirty candidate binary after the
source was reverted. The DELETE tail was the keep gate, and it failed.

## Result

Rejected and reverted. The multi-leaf backlog converted 64 separate leaf-run
flushes into retained in-memory page images, but the per-row active/backlog
probing and fallback guard did not move the benchmark in the intended
direction. The next credible DELETE design still needs a real transaction-level
many-leaf mutation representation, not a linear scanned backlog of leaf-local
runs.
