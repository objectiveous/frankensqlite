# Preserialized INSERT row-scratch borrow deferral reject

- Date: 2026-05-12T01:14:09Z
- Base commit: `a4ba59cb`
- Target: remaining fixed overhead in 100-row `INSERTThroughput`, especially
  prepared direct preserialized/prebuilt INSERT lanes.
- Candidate: in `crates/fsqlite-core/src/connection.rs`, borrow
  `prepared_direct_insert_row_scratch` only when fallback row materialization is
  needed, and use a local empty scratch for preserialized/prebuilt finish paths.
- Agent Mail: reservation could not be acquired because the local MCP HTTP
  transport at `127.0.0.1:8765/mcp/` failed.

## Correctness Gate

Command:

```bash
env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-insert-scratch-target CARGO_BUILD_JOBS=2 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture --test-threads=1
```

Result: rejected before benchmarking. The command compiled and ran the focused
test set, but failed 1/28:

```text
connection::pager_routing_tests::test_prepared_direct_simple_insert_executes_inside_explicit_transaction
```

The assertion failure was:

```text
prepared direct inserts inside BEGIN must populate the concurrent write set
```

The source patch was manually unwound. Do not retry this borrow-deferral shape
unless direct-insert page-run/write-set coupling is redesigned and the focused
explicit-transaction tests pass before benchmarking.
