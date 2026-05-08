# DML Frontier Stop-Rule Artifact - 2026-05-08

## Scope

This pass continued the current write-side performance campaign after the
direct INSERT concat candidate was rejected. It focused on the remaining
`UPDATE/DELETEThroughput` rows in the current quick matrix and asked whether a
new DML-only source patch cleared the profile-first keep gate.

Current checkout:

- `HEAD`: `f165ec05daf2c16fc0c4bae5a5559785008db581`
- Relevant current matrix artifact:
  `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/`
- Current dirty peer work was left untouched:
  `README.md` and `crates/fsqlite-e2e/src/bin/mt_mvcc_bench.rs`

## Evidence Used

The current focused DML artifact is
`tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/update-profile.json`,
with stderr stage timings in
`tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/update-profile.stderr`.

Focused DML summary:

- Faster/comparable/slower: `4/0/2`
- Average/geomean: `1.0229139149` / `1.0051237372`
- Slow rows are only the 100-row tail.

Rows:

| Scenario | C SQLite ms | FrankenSQLite ms | F/C ratio |
|---|---:|---:|---:|
| 100 rows / update 10 rows | 0.093425 | 0.114865 | 1.229489 |
| 100 rows / delete 5 rows | 0.083346 | 0.114064 | 1.368560 |
| 1000 rows / update 100 rows | 0.436948 | 0.388247 | 0.888543 |
| 1000 rows / delete 50 rows | 0.386003 | 0.343313 | 0.889405 |
| 10000 rows / update 1000 rows | 3.815117 | 3.422161 | 0.897000 |
| 10000 rows / delete 500 rows | 3.559447 | 3.077094 | 0.864487 |

The 100-row FrankenSQLite profile breaks down as:

| Row | setup us | begin us | prepare us | mutate us | commit us | direct ops |
|---|---:|---:|---:|---:|---:|---:|
| update 10/100 | 52.1 | 6.9 | 13.2 | 12.0 | 5.7 | 10 |
| delete 5/100 | 56.2 | 5.0 | 11.4 | 8.3 | 5.2 | 5 |

The mutation slice itself is small. The slow rows are dominated by fixed setup
and prepopulation work in the benchmark section, not by a large remaining
per-row DML mutation hotspot.

## Negative-Ledger Fence

The current negative-results ledger already rejects the nearby standalone DML
micro-optimizations:

- direct REAL assignment shortcut
- lazy row-scratch borrow
- private-memory direct UPDATE/DELETE page-I/O bypass
- retained-cursor/table-seek hints as standalone changes
- direct UPDATE/DELETE scratch reset removal
- fixed-width REAL payload-range and leaf-local patches
- direct DML root predecode
- SharedTxnPageIo cleanup/borrow/page-size reuse variants

Those rejects make another narrow DML-only edit low expected value unless the
candidate removes a larger setup or batching cost and wins the focused matrix
repeatedly.

## CASS Status

`cass status --json` reported an unhealthy/stale lexical index:

- last indexed: `2026-05-06T11:06:05.165+00:00`
- age: `187839` seconds
- semantic search unavailable; lexical fallback only

Targeted searches for recent DML/update/delete perf sessions returned zero
hits, so this pass treated CASS as unavailable and used the current artifacts
plus the negative ledger as the authoritative preflight.

## Blocked Probe

I attempted to build the narrow profiler with:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-dml-target CARGO_BUILD_JOBS=16 cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

The remote build failed after a cold-cache compile with:

```text
error[E0432]: unresolved import `fsqlite_wal::commit_phase_timing_forced_enabled`
```

Local source does contain and re-export that symbol in
`crates/fsqlite-wal/src/group_commit.rs` and `crates/fsqlite-wal/src/lib.rs`,
so this was not treated as evidence against current local source. No source
files were changed by this probe.

## Decision

No DML source patch was applied. The current DML evidence does not clear the
`profile-first, score >= 2.0, one lever` gate for another standalone
UPDATE/DELETE mutation micro-optimization.

The next keepable write-side patch should target one of these broader
contracts:

1. Shared fixed setup reduction that benefits both the 100-row INSERT tail and
   the 100-row UPDATE/DELETE tail in the same A/B window.
2. True DML leaf-run/page-batched mutation with read-after-write proof, not
   another per-row cursor or assignment micro-optimization.
3. Large-row page-builder work that improves `large_10col` 10K INSERT rows
   before rerunning the full quick matrix.

Do not treat the current 100-row DML rows as proof that the direct DML mutation
path itself is the next hotspot. At this point, a DML-only patch needs new
profile evidence showing a top-5 cost outside the already rejected surfaces.
