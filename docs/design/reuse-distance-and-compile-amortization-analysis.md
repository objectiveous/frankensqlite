# Reuse-Distance And Compile-Amortization Analysis

Status: F1.2 draft

This note summarizes what the current statement-reuse traces and hot-path
profiles say about compile amortization. The goal is to answer three questions:

1. Are reuse distances short enough for a small statement cache to matter?
2. How much compile time is the cache already saving on real fixture runs?
3. Is background compile/status interference large enough to matter?

## Evidence Surface

- The reuse trace records `cache_kind`, `cache_hit`, `reuse_distance`,
  `lane_locality`, `compile_ns`, and `execute_ns` in
  `crates/fsqlite-core/src/connection.rs`.
- The trace contract is emitted by `Connection::log_statement_reuse_event()`,
  which logs both compile misses and warmed execution hits.
- The in-tree trace tests prove that the current instrumentation distinguishes
  first observation, same-lane reuse, cross-lane reuse, and cross-node reuse.
- Aggregate savings estimates below use existing hot-path profile artifacts in
  `artifacts/perf/` and `.codex-bench/`.

## What The Traces Reveal

### 1. Reuse distance is short

The current trace contract explicitly records `reuse_distance` and lane
locality at the point where statement reuse is observed:

- `crates/fsqlite-core/src/connection.rs:7256`
- `crates/fsqlite-core/src/connection.rs:7294`
- `crates/fsqlite-core/src/connection.rs:7296`

The focused in-tree reuse tests show the hot statement returning after exactly
one intervening statement:

- same-lane reuse at distance `1`
- cross-lane reuse at distance `1`
- cross-node reuse at distance `1`

References:

- `crates/fsqlite-core/src/connection.rs:94768`
- `crates/fsqlite-core/src/connection.rs:94796`
- `crates/fsqlite-core/src/connection.rs:94804`
- `crates/fsqlite-core/src/connection.rs:94838`
- `crates/fsqlite-core/src/connection.rs:94840`
- `crates/fsqlite-core/src/connection.rs:94844`

Interpretation: the locality window is very short. That favors a small
lane-local LRU or per-lane compiled cache first. It does not yet justify a
heavy shared-cache design on its own.

### 2. First-hit vs warmed-hit delta is material

Using existing hot-path profiles, the cold-path extra work per statement family
is approximately:

| Profile | Avg parse miss | Avg compile miss | Cold extra per first hit |
| --- | ---: | ---: | ---: |
| `c8_mvcc` | 4.74 us | 9.83 us | 14.57 us |
| `c8_single` | 6.36 us | 13.41 us | 19.76 us |
| `c8_current` | 5.11 us | 10.15 us | 15.26 us |
| `c2_mixed` | 25.25 us | 14.69 us | 39.94 us |
| `c1_isolated` | 5.82 us | 10.40 us | 16.22 us |

Method:

- `avg_parse_miss = parse_time_ns / parse_cache_misses`
- `avg_compile_miss = compile_time_ns / compiled_cache_misses`
- `cold_extra = avg_parse_miss + avg_compile_miss`

Interpretation: a warmed hit is not saving noise. It is usually saving roughly
`15-20 us` of front-end work, and in the mixed fixture it is closer to `40 us`.

### 3. Compile amortization is already large in repeat-heavy workloads

Estimated saved compile time from existing profile bundles:

| Profile artifact | Compiled hits | Compiled misses | Avg compile miss | Estimated compile time saved | Share of potential compile work avoided |
| --- | ---: | ---: | ---: | ---: | ---: |
| `artifacts/perf/20260313_profile_drilldown/disjoint_c8_frankensqlite_mvcc.profile.json` | 455 | 9 | 9.83 us | 4.47 ms | 98.1% |
| `artifacts/perf/20260313_profile_drilldown/disjoint_c8_frankensqlite_single_writer.profile.json` | 466 | 9 | 13.41 us | 6.25 ms | 98.1% |
| `artifacts/perf/20260314_hot_profile_disjoint_c8_current/profile.json` | 463 | 9 | 10.15 us | 4.70 ms | 98.1% |
| `artifacts/perf/bd-db300.4.1/blackhill-smoke-3/runs/mvcc_c2_mixed_read_write__frankensqlite_beads/profile.json` | 101 | 109 | 14.69 us | 1.48 ms | 48.1% |

Method:

- `avg_compile_miss = compile_time_ns / compiled_cache_misses`
- `saved_compile_ns = compiled_cache_hits * avg_compile_miss`
- `saved_share = saved_compile_ns / (compile_time_ns + saved_compile_ns)`

Interpretation:

- In repeat-heavy disjoint workloads, compile amortization is already doing
  most of the possible work. The cache is not marginal there.
- In mixed fixture traffic, the cache still saves meaningful time, but the hit
  ratio is lower because the working set is broader and statement families are
  less concentrated.

### 4. The current c1 prepared path is not compile-bound

The c1 commutative profiles show a different bottleneck:

| Profile artifact | Compiled hits | Compiled misses | Prepared hits | Prepared misses | Avg compile miss | Avg schema refresh | Avg statement-dispatch background gate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `artifacts/bd-hjkbr.2/bd-hjkbr.2-20260325T042141Z-459/hotprofile_frankensqlite_commutative_c1/profile.json` | 0 | 52 | 0 | 52 | 7.17 us | 24.28 us | 0.305 us |
| `artifacts/bd-hjkbr.2/bd-hjkbr.2-20260325T042141Z-459/hotprofile_frankensearch_commutative_c1/profile.json` | 0 | 52 | 0 | 52 | 7.07 us | 23.75 us | 0.282 us |
| `artifacts/bd-hjkbr.2/bd-hjkbr.2-20260325T042141Z-459/hotprofile_frankentui_commutative_c1/profile.json` | 0 | 52 | 0 | 52 | 6.23 us | 22.38 us | 0.307 us |

Interpretation:

- These runs show zero prepared-cache and zero compiled-cache reuse.
- The per-statement schema refresh cost is roughly `3x-4x` the raw compile
  miss cost.
- Background-status gating is tiny by comparison, about `0.3 us` per
  statement-dispatch gate.

This means compile amortization alone will not move the c1 headline much until
the prepared-schema refresh path gets cheaper or more selective.

## Estimated Savings

The best current estimate is:

- Repeat-heavy disjoint workloads: the compiled cache is already saving about
  `4.5-6.3 ms` of compile time per profile run, avoiding about `98%` of the
  compile work that would otherwise be paid.
- Mixed read/write workload: compiled-cache savings are about `1.48 ms` per
  run; parse-cache savings add about `2.55 ms`, for roughly `4.03 ms` of total
  front-end work avoided.
- c1 isolated/commutative workloads: compile amortization opportunity exists
  in theory, but current measured savings are effectively zero because the path
  is missing cache hits and is dominated by schema refresh instead.

## Decision Implications For F1.3

1. Go on lane-local compile caching. The measured reuse distance is short and
   the compile-hit savings are already large in repeat-heavy fixture traffic.
2. Do not assume a shared cross-lane cache is the next win. The current
   evidence proves cross-lane reuse can happen, but the real fixture artifacts
   do not yet show that shared reuse dominates enough to justify more
   coordination.
3. Treat prepared-schema refresh as the near-term limiter on c1. In the
   current c1 profiles, that cost is larger than raw compile cost and much
   larger than background-status interference.
4. Fresh F1.1 reruns should specifically publish reuse-distance histograms by
   lane and statement family. That is the missing evidence needed before
   building a shared/background compile system.

## F1.3 Go/No-Go Memo

This section is the explicit closure memo for `bd-db300.6.1.3`.

### Decision Summary

| Candidate | Decision | Why |
| --- | --- | --- |
| Per-lane / lane-local compiled cache | **GO** | The measured reuse window is short, warmed-hit savings are material, and repeat-heavy workloads already realize most of the benefit without cross-lane coordination. |
| Shared cross-lane compiled cache | **NO-GO for now** | The current evidence does not show enough cross-lane reuse to justify extra coordination, invalidation scope, or ownership complexity. |
| Background compile cache / speculative shared compile service | **NO-GO for F3 right now** | The c1 bottleneck is schema refresh, not compile time. Adding a shared background compile system now would optimize the wrong cost center. |

### Explicit Recommendation For F2

F2 should assume **locality-first compiled reuse**, not a shared cache.

That means:

- keep compiled/program reuse scoped to the current connection or lane,
- prefer small lane-local structures and cheap invalidation,
- avoid new shared synchronization on the current statement-execution path,
- treat cross-lane reuse as opportunistic evidence gathering, not as a design
  requirement.

### Explicit Recommendation For F3

F3 should treat the shared/background compile cache as **rejected pending new
evidence**.

That means:

- do not build a shared compiled-cache service yet,
- do not add coordination solely to chase hypothetical cross-lane reuse,
- reopen the decision only after fresh reruns prove that cross-lane reuse is
  both common and large enough to dominate the remaining front-end cost after
  schema-refresh work lands.

### Reopen Conditions

This memo should be revisited only if one of the following becomes true:

1. fresh F1.1 reruns show a large cross-lane reuse share by statement family,
2. schema-refresh costs fall far enough that compile cost becomes the next
   dominant c1 front-end component,
3. a later workload family shows repeated misses that a shared cache would
   clearly eliminate without increasing contention on the execute path.

### Net Call

The correct call from the current evidence is:

- **GO** on lane-local compiled caching and reuse as the default mental model
  for F2.
- **NO-GO** on shared/background compile caching for F3 until new locality data
  proves that the added coordination is worth paying for.

## Fresh Rerun Status

I attempted a fresh trace rerun with:

`rch exec -- env RUN_ID=bd-db300-6-1-2-trace TRACE_ID=bd-db300-6-1-2-trace SCENARIO_ID=STATEMENT-REUSE-F1-2 RUST_TEST_THREADS=1 NO_COLOR=1 RUST_LOG=fsqlite.statement_reuse=info cargo test -p fsqlite-core statement_reuse_regression_file_backed_trace_contract -- --nocapture`

That rerun is currently blocked by unrelated `fsqlite-mvcc` compile failures in
`crates/fsqlite-mvcc/src/writer_routing_telemetry.rs`, so the analysis above is
grounded in the committed trace contract and existing artifact bundles rather
than a fresh runtime capture.
