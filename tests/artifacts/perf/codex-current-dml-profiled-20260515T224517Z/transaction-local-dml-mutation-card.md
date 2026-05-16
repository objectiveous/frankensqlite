# Transaction-Local DML Mutation Operator Card

- Date: 2026-05-15
- Source profile artifact:
  `tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/summary.md`
- Source checkout during measurement:
  `06a37f61e0ad97ffa95449f2f97a27ea080c821c`
- Worktree state during measurement: dirty, with ALTER TABLE rename repairs in
  progress. The profile still validates the DELETE fast-path shape because the
  measured DML rows stayed on prepared direct DELETE with `slow=0` and
  `vdbe_opcodes=0`.
- Baseline comparator: legacy C SQLite in
  `comprehensive-bench --quick --filter update-delete`.

## Decision

No source patch should be landed from this profile alone. The current evidence
does not expose a new one-function retained-run optimization. It reinforces the
existing frontier: the only credible DELETE source attempt is a transaction-local
DML mutation operator that buffers logical rowid/key messages, proves
read-boundary semantics, groups by B-tree leaf, mutates each dirty leaf once,
and publishes the normal MVCC conflict surface.

This is not the rejected prepared DELETE logical rowid/keyspace buffer from
2026-05-11. That attempt mainly tried to return affected counts from a logical
view and defer physical work. The viable lever here must also own the grouped
physical leaf mutation plan, read-your-writes boundary, rollback/savepoint
ownership, cache invalidation, and MVCC publication proof.

## Hotspot Table

| Rank | Location | Metric | Value | Category | Evidence |
|---:|---|---:|---:|---|---|
| 1 | prepared direct DELETE row | scenario ratio | `2.61x` F/C for 1000/delete50; `1.60x` F/C for 10000/delete500 | end-to-end latency | `summary.md` scenario table |
| 2 | retained leaf-run flush/materialize | time/count | `delete_leaf_flush=64/64`, `delete_leaf_flush_ns=108954`, `delete_leaf_materialize=64/86529ns` | page-local mutation/publication | `summary.md` DML profile highlights |
| 3 | retained leaf search | time/count | `delete_leaf_search=560/89746ns` | CPU | `summary.md` DML profile highlights |
| 4 | same-leaf active path | count | `delete_leaf_active=433/496`, `delete_leaf_dupcheck=500/18966ns`, `delete_leaf_compact=497/22780ns`, `delete_leaf_cellparse=497/19628ns` | CPU/leaf ceremony | `summary.md` DML profile highlights |
| 5 | leaf-boundary churn | count | `delete_leaf_miss=63`, including `60` out-of-leaf and `3` last-cell misses | B-tree traversal/publish granularity | `summary.md` DML profile highlights |

The profile rejects two common misreads:

- `setup_us` is fixture prepopulation outside the measured update/delete row.
- The DELETE gap is not VDBE fallback: `direct_delete=500`, `slow=0`,
  `vdbe_opcodes=0` in the 10000/delete500 row.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Verdict |
|---|---:|---:|---:|---:|---|
| Full transaction-local DML mutation operator with grouped leaf flush | 5 | 3 | 5 | 3.0 | viable |
| Direct `CellVisibilityLog::record_delete` hook from the current direct path | 2 | 2 | 4 | 1.0 | reject |
| Another retained `TableLeafDeleteRun` search/admission/materializer tweak | 1 | 2 | 2 | 1.0 | reject |
| Affected-count-only logical rowid/keyspace buffer | 1 | 1 | 3 | 0.3 | reject |

Score formula: `impact * confidence / effort`. Only the first candidate clears
the `>= 2.0` optimization gate. It is also the only candidate that is materially
different from the negative-ledger fences.

## Graveyard Match

- B-epsilon tree message buffers map to logical DML messages that are batched
  and flushed downward instead of applying every row mutation immediately.
- Bw-tree delta records map to representing changes as logical deltas, but the
  design must avoid unbounded delta-chain reads; consolidation at read or commit
  boundaries is mandatory.
- Differential dataflow style deltas map to proof-friendly operation deltas:
  the operator should carry deletes/updates as exact key-space facts until the
  boundary where the physical page image is required.

The current FrankenSQLite fit is a bounded transaction-local operator, not a
new global index structure.

## Implementation Contract

1. Admission is limited to prepared direct-simple DML shapes that already bypass
   VDBE and do not involve triggers, foreign keys, RETURNING, recursive effects,
   or schema-changing statements.
2. Each transaction owns per-root logical mutation buffers keyed by stable rowid
   or index key, never physical cell index.
3. Affected-row counts must come from a proven existence oracle. Returning `1`
   from a stale private `MemDatabase` mirror or an approximate key certificate is
   not allowed; absent rowids must return `0` exactly.
4. Duplicate mutations in one transaction collapse with SQLite-compatible
   effects. DELETE after DELETE returns `0` on the second statement; UPDATE after
   DELETE observes the missing row unless a prior INSERT restored it.
5. Before any read can observe the target table, dependent index, schema object,
   VDBE fallback, trigger body, foreign-key check, or user-visible result, the
   operator either flushes the logical mutations into physical pages or serves a
   delta-aware read view with the same rows SQLite would expose.
6. Flush sorts or otherwise groups messages by B-tree leaf, walks each touched
   leaf once, applies all same-leaf rowid mutations while the leaf image is
   staged, and publishes at most one dirty page write per dirty leaf when the
   B-tree structural constraints allow it.
7. Structural fallback is mandatory for last-cell deletion, empty non-root leaf,
   overflow payloads, page split/merge pressure, separator repair, or any case
   where the grouped operator cannot prove the same B-tree invariants as the
   existing physical path.
8. Savepoints and rollback own buffer checkpoints. Rolling back to a savepoint
   must remove only mutations newer than that savepoint and must restore affected
   count/read-view behavior.
9. QF membership, row-count/count-cache, `MemDatabase` mirrors, and prepared
   statement/schema proof caches must be invalidated or updated at the same
   logical boundary as the existing direct physical path.
10. MVCC publication and conflict tracking must include the same touched page
    set as the equivalent physical DELETE/UPDATE path. Concurrent writer defaults
    in `Connection`, `HarnessSettings`, `FsqliteExecConfig`, and fairness
    benchmark settings must remain true.

## Proof Obligations

- Oracle tests against `rusqlite` for affected row counts on present, missing,
  duplicate, ascending, descending, and mixed rowid sequences.
- Read-your-writes tests for SELECT, UPDATE, DELETE, INSERT, trigger execution,
  FK checks, and VDBE fallback after staged logical mutations.
- Savepoint and rollback tests covering nested savepoints and mixed
  UPDATE/DELETE/INSERT on the same rowid.
- B-tree invariant tests for grouped flush across root leaf, non-root leaf,
  leaf-boundary crossing, last-cell fallback, empty-leaf fallback, overflow
  payloads, and separator repair fallback.
- MVCC/concurrent tests proving page conflict tracking and snapshot visibility
  match the existing physical path.
- Cache invalidation tests for QF, row counts, `MemDatabase`, schema proofs, and
  prepared statement reuse.

## Acceptance Gates

Focused proof:

```text
rch exec -- cargo test -p fsqlite-core --lib test_prepared_direct_delete_leaf_run -- --nocapture --test-threads=1
rch exec -- cargo test -p fsqlite-core --lib transaction_local_dml -- --nocapture --test-threads=1
```

Focused performance:

```text
rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/<run-id>/update-delete.json --no-html
```

Keep only if both focused DML runs improve the absolute FrankenSQLite medians
for 100/delete5, 1000/delete50, and 10000/delete500 without regressing UPDATE.
Then run the full quick matrix twice and keep only if the primary weighted score
is neutral or better with no new critical red rows.

Workspace gates after code:

```text
rch exec -- cargo check --workspace --all-targets
rch exec -- cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
ubs <changed rust files>
```

## Anti-Goals

Do not retry these from the current evidence:

- standalone retained DELETE search, duplicate-check, compactness, or
  materialization tuning;
- direct flush/publication wrappers that still publish one retained run at a
  time;
- cancellation polling weakening;
- per-connection synced-write caches;
- tombstone-only overlays that do not prove read-boundary semantics;
- affected-count-only logical DELETE buffers.

## Rollback

The operator must land as one optimization lever. If any proof or performance
gate fails, revert that single source commit and keep this artifact plus the new
negative-ledger entry. No off-by-default compatibility shim should remain.
