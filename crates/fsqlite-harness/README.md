# fsqlite-harness

Conformance test runner and verification harness for FrankenSQLite. This crate is **not published** (`publish = false`).

## Overview

This crate is the central verification and testing infrastructure for the FrankenSQLite project. It is intentionally more than "just tests" -- it contains reusable verification tooling, trace exporters, schedule exploration harnesses, and parity checking that other crates can call into from their own test suites.

The harness provides a wide range of verification capabilities including differential testing against C SQLite (via rusqlite), crash recovery parity verification, concurrent writer parity, WAL/journal parity, extension parity matrices, metamorphic testing, adversarial search, soak testing, and formal TLA+ model integration. It also includes CI gate infrastructure, coverage enforcement, and release certification.

This crate sits at the top of the fsqlite workspace dependency graph for testing purposes. It depends on the core `fsqlite` crate, `fsqlite-vfs`, `fsqlite-types`, `fsqlite-error`, and numerous utility crates. In dev-dependencies, it pulls in nearly every fsqlite crate for comprehensive cross-crate testing.

## Key Modules

- `differential_runner` / `differential_v2` - Run identical SQL against FrankenSQLite and C SQLite, comparing results
- `oracle` - Reference oracle for expected query results
- `crash_recovery_parity` / `cross_process_crash_harness` - Verify crash recovery behavior matches C SQLite
- `concurrent_writer_parity` / `lock_txn_parity` - Verify concurrent access and transaction semantics
- `wal_journal_parity` - Verify WAL and journal mode behavior parity
- `fault_vfs` / `fault_profiles` - Fault injection VFS for testing error handling paths
- `metamorphic` - Metamorphic testing: transform queries and verify invariant results
- `adversarial_search` - Adversarial/fuzz-like test generation
- `extension_parity_matrix` - Verify extension behavior matches C SQLite
- `soak_executor` / `soak_profiles` - Long-running soak tests
- `tcl_conformance` - TCL test suite conformance checking
- `ci_gate_matrix` / `ci_coverage_gate` / `confidence_gates` - CI pipeline quality gates
- `release_certificate` - Generate release readiness certificates
- `replay_harness` / `replay_triage` - Record and replay test executions
- `tla` - TLA+ formal model integration
- `bloodstream` / `impact_graph` / `closure_wave` - Dependency and impact analysis
- `benchmark_corpus` / `perf_loop` - Performance benchmarking infrastructure

## Reusable Primitives for Swarm / E2E Consumers

Downstream crates (`fsqlite-e2e`, future `tests/swarm/`) should import these
primitives rather than reinventing them.

### Fixture & Corpus Management

| Module | Key Types | Use For |
|--------|-----------|---------|
| `fixture_discovery` | `DiscoveryConfig`, `Candidate` | Locating SQLite files with safe bounded traversal |
| `fixture_root_contract` | `FixtureRootContract` | Enforcing fixture/SLT root cardinality contracts |
| `unit_fixtures` | `FixtureSeed`, `FixtureCatalog` | Deterministic seed derivation (`xxh3_64` with domain tags) |
| `oracle` | `TestFixture`, `FixtureResult` | Reference oracle fixture and result types |
| `corpus_ingest` | `CorpusEntry`, `CorpusManifest`, `Family` | Corpus taxonomy (8 families) and content-hash IDs |

### Structured Logging & Schema Validation

| Module | Key Types | Use For |
|--------|-----------|---------|
| `e2e_log_schema` | `LogEventSchema`, `LogPhase`, `LogEventType` | Unified event schema (v1.0.0) with forward compatibility |
| `log_schema_validator` | `validate_event_stream()`, `redact_event()` | Batch JSONL validation and deterministic redaction |
| `log` | `HarnessEvent`, `ReproBundle`, `BundleMeta` | Lifecycle events and reproducibility bundles |
| `e2e_logging_init` | `RunContext`, `E2eLoggingConfig` | Per-run correlation context and output format presets |

### Cross-Process & Crash Testing

| Module | Key Types | Use For |
|--------|-----------|---------|
| `cross_process_crash_harness` | `ProcessRole`, `CrashPoint`, `StructuredCrashEvent` | 4-role × 5-crash-point deterministic crash matrix |
| `eprocess` | `MvccInvariant` (INV-1..INV-7) | E-process calibration for MVCC invariant monitoring |
| `e2e_orchestrator` | `ExecutionManifest`, `ManifestEntry`, `RetryPolicy` | Script scheduling with phase ordering and retry |

### Concurrent Writer Verification

| Module | Key Types | Use For |
|--------|-----------|---------|
| `concurrent_writer_parity` | `ConcurrentInvariantArea`, `ConcurrentWriterParityReport` | 10-area parity assessment (5 critical) |
| `soak_profiles` | `ContentionMix`, `SoakProfile`, `SoakWorkloadSpec` | Long-run workload profiles with reader/writer mix |

### Seed Management

| Module | Key Types | Use For |
|--------|-----------|---------|
| `seed_taxonomy` | `SeedTaxonomy` | 4-purpose seed derivation (schedule, entropy, fault, fuzz) |
| `unit_fixtures` | `FixtureSeed` | Domain-tagged seed derivation with child seeds |

### Complementary Types in fsqlite-e2e (not duplicated here)

These types live in `fsqlite-e2e` and serve different purposes:

- `OperationMix` — DML operation weights (insert/update/delete/select), vs `ContentionMix` which is thread-role distribution
- `FixtureMetadataV1` / `FixtureSafetyV1` — PRAGMA-extracted metadata and CI safety classification
- `TraceContext` / `OpEvent` — per-operation structured tracing (bd-zywqc.1), vs `LogEventSchema` which is test-lifecycle events
- `FRANKEN_SEED` / `derive_worker_seed` — same base seed as `CORPUS_SEED_BASE`, different derivation API

## Agent-Swarm Replay Lab

The `agent_swarm_trace` module is the operator path for sanitized multi-agent SQL traces. It owns the trace schema, SQL literal scrubber, deterministic replay harness, resource scorecard, evidence manifest, and the fast CI smoke artifact.

Fast CI smoke replay:

```bash
cargo test -p fsqlite-harness --lib agent_swarm_ci_smoke -- --nocapture
```

Heavy replay and lint checks must stay offloaded through `rch`:

```bash
timeout 1200 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-agent-swarm-replay cargo test -p fsqlite-harness --lib agent_swarm_replay -- --nocapture
timeout 1200 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-agent-swarm-trace cargo test -p fsqlite-harness --lib agent_swarm_trace -- --nocapture
timeout 1200 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-agent-swarm-clippy cargo clippy -p fsqlite-harness --lib --no-deps -- -D warnings
```

Minimal capture/scrub/replay path:

1. Capture raw statement metadata from Agent Mail, Beads, or CASS into `RawTraceStatement` rows with deterministic `logical_order`, logical actor ids, connection ids, transaction ids, concurrency groups, and expected result classes.
2. Convert each row with `TraceStatement::from_raw`; this preserves topology while replacing private string, numeric, blob, and SQL-comment literals with scrubber placeholders.
3. Store or load JSON through `load_agent_swarm_trace_json` or `load_agent_swarm_trace_file` so the schema version and scrubber version are validated before replay.
4. Replay with `replay_agent_swarm_trace` or `replay_agent_swarm_trace_with_executors`; FrankenSQLite remains on the concurrent-writer default path and C SQLite remains the oracle backend.
5. Score with `score_agent_swarm_resource_envelope`, then build `AgentSwarmEvidenceManifest` and `AgentSwarmCiSmokeArtifact` so operators get a trace hash, replay command, backend metrics, minimized first-failure slice, and duplicate-regression hints.

Key metrics:

- `throughput_statements_per_second_x1000` records statement throughput in fixed-point form for artifact comparison only.
- `latency_p50_ns`, `latency_p95_ns`, and `latency_p99_ns` capture replay latency distribution.
- `abort_count`, `retry_count`, `expected_mismatch_count`, and `conflict_classes` separate expected busy/conflict behavior from unexpected failures.
- `memory_high_water_bytes`, `cpu_utilization_per_mille`, `page_cache_utilization_per_mille`, and `fairness_index_per_mille` describe the selected resource profile.
- `first_failure_diag` is copied into replay reports, scorecards, evidence manifests, CI smoke artifacts, and structured logs.

Every final smoke artifact and structured CI-smoke log row includes `trace_id`, `run_id`, `scenario_id`, `command`, `backend`, `profile_id`, `artifact_manifest_path`, and `first_failure_diag`. Numeric performance claims in the root README still require a benchmark or artifact path, commit, and run date; scorecards and smoke artifacts alone are not permission to write uncited throughput or speedup claims.

## Dependencies (runtime)

- `fsqlite`, `fsqlite-error`, `fsqlite-types`, `fsqlite-vfs`
- `asupersync`, `serde`, `serde_json`, `sha2`, `xxhash-rust`, `parking_lot`, `tracing`

## Dependencies (dev)

- All fsqlite extension crates (`fsqlite-ext-json`, `fsqlite-ext-fts5`, `fsqlite-ext-fts3`, `fsqlite-ext-rtree`, `fsqlite-ext-session`, `fsqlite-ext-icu`, `fsqlite-ext-misc`)
- Core crates: `fsqlite-ast`, `fsqlite-btree`, `fsqlite-core`, `fsqlite-func`, `fsqlite-mvcc`, `fsqlite-pager`, `fsqlite-parser`, `fsqlite-planner`, `fsqlite-vdbe`, `fsqlite-wal`
- Testing utilities: `blake3`, `proptest`, `tempfile`, `toml`, `trybuild`, `rusqlite`

## License

MIT
