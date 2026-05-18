# 2026-05-17 INSERT concat itoa buffer candidate

Context:
- Candidate source changed `crates/fsqlite-core/src/connection.rs` to reuse one
  `itoa::Buffer` per prepared direct INSERT concat-chain evaluation.
- A fresh-eyes pass caught and removed an unused wrapper left by the first
  version of the candidate before measuring it.
- The source candidate was manually unwound after the same-window A/B below.
- `/tmp` was full, so both runs used `TMPDIR=/data/tmp`.
- `rch` fell back locally because workers were under critical pressure.

Commands:

```bash
rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-insert-itofmt-candidate-20260517 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-insert-itofmt-buffer-candidate-20260517Tnext/insert.json \
  --no-html

rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-insert-itofmt-candidate-20260517 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-insert-itofmt-baseline-samewindow-20260517Tnext/insert.json \
  --no-html
```

Candidate result:
- Total scenarios: 25.
- FrankenSQLite faster: 15; comparable: 2; C SQLite faster: 8.
- Average ratio: `1.0505x`; geomean: `0.9662x`; weighted score:
  `0.8547x`; p90: `1.6336x`; p99: `2.8338x`.

Restored-original same-window baseline:
- Total scenarios: 25.
- FrankenSQLite faster: 20; comparable: 1; C SQLite faster: 4.
- Average ratio: `0.8451x`; geomean: `0.8110x`; weighted score:
  `0.7837x`; p90: `1.1633x`; p99: `1.9335x`.

Decision:
- Rejected and unwound. Lower ratios are better, and the restored-original
  baseline wins the same-window INSERT quick matrix.
- Do not retry standalone shared `itoa::Buffer` reuse in concat text assembly.
