# 2026-05-18 DELETE count/sum no-op trim candidate

Context:
- Candidate source removed two `retained_autocommit_count_sum_cache_note_delete`
  calls from the retained same-leaf prepared DELETE path.
- The source patch was manually unwound after this focused benchmark rejected
  it.
- `rch` fell back locally because workers were under critical pressure.

Candidate command:

```bash
rch exec -- env TMPDIR=/data/tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-update-delete-20260518 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update-delete \
  --json-out tests/artifacts/perf/codex-delete-countsum-noop-trim-candidate-20260518Tnext/update-delete.json \
  --no-html
```

Correctness proof before measurement:

```bash
rch exec -- env TMPDIR=/data/tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-countcache-trim-test \
  cargo test -p fsqlite-core pending_direct_delete_leaf_run -- --nocapture
```

Result:
- Rejected. The candidate did not improve all DELETE rows in absolute
  FrankenSQLite time against the same-window current refresh in
  `tests/artifacts/perf/codex-current-update-delete-refresh-20260518Tnext/`.
- `100 rows / delete 5 rows`: F `0.024406 ms -> 0.015697 ms`, but both sides
  were high variance.
- `1000 rows / delete 50 rows`: F `0.038653 ms -> 0.035671 ms`.
- `10000 rows / delete 500 rows`: F `0.276107 ms -> 0.327832 ms`, a clear
  regression on the stable large row.

Decision:
- Do not keep the source patch.
- Do not retry retained same-leaf DELETE count/sum-cache no-op trimming as a
  standalone optimization. The retained count/sum cache is already absent for
  this activation shape, and removing the maintenance call is too small/noisy
  to clear the focused DELETE keep gate.
