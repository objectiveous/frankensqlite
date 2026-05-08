# Prebuilt empty-root leaf page-run candidate

- Date: 2026-05-08
- Agent: SwiftGate
- Baseline: `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/insert-profile.json`
- Candidate artifacts:
  - `candidate-insert.json`
  - `candidate-insert-repeat.json`
  - `candidate-insert.html`
  - `candidate-insert-repeat.html`
  - `stdout/candidate-insert.stdout`
  - `stdout/candidate-insert.stderr`
  - `stdout/candidate-insert-repeat.stdout`
  - `stdout/candidate-insert-repeat.stderr`

## Candidate

The rejected source patch added a hidden btree helper that prebuilt no-overflow
table leaf pages for monotonic prepared direct INSERT page-runs that start from
an empty non-page-1 root. `Connection` routed only large records
(`record_bytes.len() >= 384`) into that helper; small rows, non-empty right-edge
batches, repeated records, autocommit, and existing owned/arena paths were left
unchanged. The goal was to reduce large-row btree insert and commit pressure
without repeating earlier standalone expression-serializer or admission tweaks.

## Correctness Proofs Before Rejection

- `cargo test -p fsqlite-btree test_table_bulk_load_empty_root_prebuilt_leaf_pages_builds_reachable_tree -- --nocapture`
- `cargo test -p fsqlite-core test_prepared_direct_insert_large_empty_page_run_uses_prebuilt_leaves -- --nocapture`
- `cargo fmt --check -p fsqlite-btree -p fsqlite-core`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-prebuilt-leaf-check-target cargo check -p fsqlite-btree -p fsqlite-core --lib`

## Benchmark Result

The first insert-filter candidate run showed one promising target-row win but a
bad overall insert distribution:

- Frontier insert summary: faster/comparable/C-faster `17/2/6`, average ratio
  `0.803142`, geomean `0.780274`, median `0.725773`, p90 `1.074184`, p99
  `1.132336`, weighted score `0.788869`.
- Candidate first run: faster/comparable/C-faster `16/2/7`, average ratio
  `0.850594`, geomean `0.825594`, median `0.810453`, p90 `1.133610`, p99
  `1.150862`, weighted score `0.879814`.
- Candidate repeat: faster/comparable/C-faster `19/1/5`, average ratio
  `0.841602`, geomean `0.815917`, median `0.805487`, p90 `1.144750`, p99
  `1.388409`, weighted score `0.808149`.

The target 10K large-record row was unstable:

- `record-size large_10col 10K`: frontier F `10.528151 ms`; first candidate
  F `9.107507 ms`; repeat candidate F `12.076303 ms`.
- `single-txn large_10col 10K`: frontier F `10.262022 ms`; first candidate
  F `10.036465 ms`; repeat candidate F `9.005495 ms`.

## Decision

Rejected. The repeat still lost the insert keep gate against the frontier, and
the exact target row that motivated the candidate did not hold up on repeat.
The source patch was manually unwound; these artifacts and the negative ledger
entry are the durable result.
