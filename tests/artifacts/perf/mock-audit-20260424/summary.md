# `/mock-code-finder` audit: `fsqlite-pager` + `fsqlite-wal` (2026-04-24)

Scope: `crates/fsqlite-pager/src/` + `crates/fsqlite-wal/src/`.
HEAD at audit: post-0ee66492 (`clear_page_data` hint hot-path).
Trigger: user pivot after perf campaign plateaued at 3.24× C sqlite
at 8t, 1.04× at 4t. Looking for stubs, placeholder code, TODOs, or
unused-abstraction carry-overs that were hidden by the perf noise.

## Summary

- **Two modules are pub-exported but have ZERO production consumers
  anywhere in the workspace** (~4.2k LOC of carry-over). Filed as
  P1/P2 bugs for user disposition because AGENTS.md Rule 1 forbids
  file deletion without explicit permission.
- **No runtime stubs found.** No `todo!()`, no `unimplemented!()`,
  no panics-in-production-paths in either crate. The two `panic!`s
  in `wal/telemetry.rs:429` and `pager/s3_fifo.rs:2478` are both
  inside `#[cfg(test)]` scopes.
- **No TODO/FIXME/XXX/HACK comments.** Clean.
- **Default-Ok trait fallbacks verified.** The
  `WalBackend` trait in `fsqlite-pager/src/traits.rs` has several
  default impls that return `Ok(0)` / `Ok(Vec::new())` / `Ok(None)`
  (lines 223, 296, 313, 323, 742, 977, 1046, …). Each one is
  properly overridden in `fsqlite-core/src/wal_adapter.rs` by the
  production `WalBackendAdapter` — the defaults exist only for
  trivial / test backends. Not stubs.
- **"Bootstrap stub" at pager.rs:5298 is documentation.** The comment
  describes a legitimate SQLite WAL-recovery branch (stale main
  header + live WAL); the implementation behind it
  (`bootstrap_header_from_stale_main_file`, pager.rs:4147) is real
  production code.

## Findings filed as beads

| Bead | Priority | Finding | LOC |
|:-:|:-:|---|---:|
| `bd-q7zls` | P1 bug | `fsqlite-pager::arc_cache` module is pub-exported but has zero production consumers anywhere. Only mention outside the module itself is in the spec doc. The active cache path is `ShardedPageCache` / `FlatPageSlots`, not ARC. | **3,866** |
| `bd-5ftij` | P2 bug | `fsqlite-pager::thompson_partitioner` module is pub-exported but has zero production consumers. Beta-Bernoulli sampler for cache-policy selection that was never wired. | **350** |

Each bead includes three disposition options (delete / wire / gate
behind an experimental feature flag) for the user to choose from.
Flagged as P1 for `arc_cache` because of the size (3.9k LOC + test
suite), P2 for the partitioner.

## Modules I verified are LIVE (not dead)

Cross-checked that these pub-exported modules DO have production
consumers:

- `s3_fifo` — used from `page_cache.rs` (`S3Fifo`, `S3FifoConfig`,
  `S3FifoEvent`).
- `evalue_eviction` — wired into `ShardedPageCache::evalue_evictor`
  (`page_cache.rs:1916 / 2051 / 2172 / 2202`).
- `submodular_prefetch` — called from `pager.rs:6082 / 6086`
  (`greedy_select`, `expected_gain`, `Candidate`).
- All `checkpoint_*` / `native_commit` / `parallel_wal` /
  `per_core_buffer` / `recovery_compaction` / `recovery_fence` /
  `telemetry` / `wal_fec` / `wal_index` — live.

## Other soft signals (not filed; may be noise)

- `arc_cache.rs:846` and `arc_cache.rs:1297` both
  `#[allow(dead_code)]` inside the dead module — self-consistent,
  folded into `bd-q7zls`.
- `pager.rs:88` `#[allow(dead_code)]` on `LegacyCondvarTimeout` —
  legitimate: it's a compile-time alternative to `KeyedEventcount`
  preserved for rollback, annotated with that intent.
- `fault_hooks` modules in both crates: test-only machinery, gated
  under `#[cfg(any(test, feature = "fault-injection"))]` in both
  places. Not dead.
- One `#[ignore]` at `pager.rs:25319` is a test that requires a
  real filesystem — legitimate, documented.

## Methodology

Executed searches in this order; each stanza filtered out tests,
backup files, docs/md, and target dirs:

```
# TODOs / FIXMEs / runtime stubs
grep -rn "TODO\|FIXME\|XXX\|HACK" ...
grep -rnE "unimplemented!|todo!\(|panic!\(.*(not|unimplemented|todo)" ...

# stub / placeholder / dummy / fake / mock words
grep -rnE '\b(stub|placeholder|dummy|fake|mock)\b' ...

# "not implemented" / "not supported" / no-op Result returns
grep -rnE "not yet|not (yet )?implemented|not (yet )?supported|^[[:space:]]*Ok\(\(\)\)[[:space:]]*$" ...

# dead_code allows outside #[cfg(test)] blocks
grep -rn "#\[allow(dead_code)\]\|#\[ignore\]" ...

# suspiciously small Result-returning fns (Ok(None), Ok(0), Ok(Vec::new()))
grep -rnE "^[[:space:]]*Ok\(Vec::new\(\)\)$|^[[:space:]]*Ok\(None\)$|^[[:space:]]*Ok\(0\)$" ...
```

Cross-referenced each "suspect" module against
`grep -rn "ModuleName::" ...` to confirm it had no production
consumers before filing.

## Disposition

- `bd-q7zls` (arc_cache): open for user disposition (delete / wire /
  feature-gate).
- `bd-5ftij` (thompson_partitioner): open for user disposition
  (same three options).
- No other cleanup warranted in these two crates — both are well-
  maintained and free of the usual "stub that became permanent"
  pattern.
- If the user picks "delete" on either, a follow-up commit removes
  the module, the `pub mod` + `pub use` lines, and runs
  `cargo check --workspace` to confirm no outside consumer exists
  (the audit already confirmed this, but the build is the ground
  truth).
