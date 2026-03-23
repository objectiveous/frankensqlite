# Evidence Ownership, Rerun Entrypoints, Compatibility Selectors, and Gap-Conversion Rules

**Bead:** `bd-db300.7.5.3` (G5.3)
**Date:** 2026-03-23
**Status:** Operational contract
**Dependency gap:** G5.2 (crash/fault mapping) is OPEN; CRF rows are included provisionally.

---

## 1. Evidence Registry

Every verification obligation in the performance program maps to exactly one
row in this registry. Each row is a binding contract: the named owner must
produce the named evidence bundle via the named rerun entrypoint, and CI must
be able to verify the pass/fail signature mechanically.

### 1.1 Registry Schema

```
claim_id          — What is being claimed (bead ID or scorecard cell)
evidence_id       — Unique key for the evidence bundle (scenario_id:mode:placement:concurrency:seed)
verification_class— COR | CRF | RBR | TOP | PFA (from G7.2)
owner             — Agent or team responsible for producing evidence
evidence_bundle   — artifact_root path or manifest key (from G6.3)
rerun_entrypoint  — Exact command to reproduce (from verify-suite or cargo test)
compatibility_surface — What is being compared (row_level | integrity_check | hot_path_profile | topology_metrics)
oracle            — Reference implementation (rusqlite | wal_invariant | self_consistency | none)
allowed_diff_policy — What differences are acceptable (none | numeric_epsilon_1e-10 | ordering_only)
baseline_comparator — What the result is compared against (sqlite3_c1_baseline | previous_commit | none)
pass_fail_signature — How pass/fail is determined (sha256 of sorted comparison | artifact_present | threshold)
quick_or_full     — Whether this is Quick-suite mandatory or Full-only
policy_id         — Controller policy version if adaptive behavior involved (e4.3.v1 | dro.v1 | none)
decision_id       — Runtime decision trace join key if controller is active (trace_id:decision_counter | none)
budget_id         — Budget or SLO identifier if budgeted (lane_2_evidence | gc_chain_depth | none)
shadow_lineage    — Shadow-run comparator if shadow mode was active (conservative_baseline_run_id | none)
fallback_lineage  — What fallback was active during evidence collection (safe_mode | none)
```

### 1.2 Registry Rows — Correctness (COR)

| claim_id | evidence_id | owner | rerun_entrypoint | oracle | allowed_diff | baseline | quick |
|----------|-------------|-------|------------------|--------|-------------|----------|-------|
| bd-db300.9.12 | REF-COR-01:sqlite_reference:baseline_unpinned:c1:42 | suite-runner | `realdb-e2e verify-suite --scenario REF-COR-01 --depth quick` | rusqlite | none | sqlite3_c1_baseline | Quick |
| bd-db300.9.12 | REF-COR-02:sqlite_reference:baseline_unpinned:c1:42 | suite-runner | `realdb-e2e verify-suite --scenario REF-COR-02 --depth quick` | rusqlite | none | sqlite3_c1_baseline | Quick |
| bd-db300.9.12 | MVCC-COR-01:fsqlite_mvcc:baseline_unpinned:c4:42 | suite-runner | `realdb-e2e verify-suite --scenario MVCC-COR-01 --depth quick` | rusqlite | none | sqlite3_c1_baseline | Quick |
| bd-db300.9.12 | MVCC-COR-02:fsqlite_mvcc:baseline_unpinned:c4:42 | suite-runner | `realdb-e2e verify-suite --scenario MVCC-COR-02 --depth quick` | rusqlite | none | sqlite3_c1_baseline | Quick |
| bd-db300.9.12 | MVCC-COR-03:fsqlite_mvcc:baseline_unpinned:c2:42 | suite-runner | `realdb-e2e verify-suite --scenario MVCC-COR-03 --depth quick` | rusqlite | none | sqlite3_c1_baseline | Quick |
| bd-db300.9.12 | MVCC-COR-05:fsqlite_mvcc:recommended_pinned:c8:42 | suite-runner | `realdb-e2e verify-suite --scenario MVCC-COR-05 --depth full` | rusqlite | none | sqlite3_c1_baseline | Full |
| bd-db300.9.12 | SW-COR-01:fsqlite_single_writer:baseline_unpinned:c1:42 | suite-runner | `realdb-e2e verify-suite --scenario SW-COR-01 --depth quick` | rusqlite | none | sqlite3_c1_baseline | Quick |

### 1.3 Registry Rows — Performance Attribution (PFA)

| claim_id | evidence_id | owner | rerun_entrypoint | oracle | baseline | quick | policy_id | budget_id |
|----------|-------------|-------|------------------|--------|----------|-------|-----------|-----------|
| bd-db300.4.3.1 | MVCC-PFA-01:fsqlite_mvcc:baseline_unpinned:c1:42 | perf-runner | `realdb-e2e hot-profile --workload commutative_inserts --concurrency 1` | none | sqlite3_c1_baseline | Full | none | none |
| bd-db300.4.4.1 | MVCC-PFA-02:fsqlite_mvcc:recommended_pinned:c4:42 | perf-runner | `realdb-e2e hot-profile --workload commutative_inserts --concurrency 4` | none | sqlite3_c1_baseline | Full | none | none |
| bd-db300.5.4.3 | MVCC-PFA-04:fsqlite_mvcc:recommended_pinned:c4:42 | perf-runner | `realdb-e2e hot-profile --workload hot_page_contention --concurrency 4` | none | sqlite3_c1_baseline | Full | e4.3.v1 | lane_0_writer |
| bd-3t52f | MVCC-PFA-DRO:fsqlite_mvcc:baseline_unpinned:c4:42 | perf-runner | `cargo test -p fsqlite-mvcc test_dro_adversarial -- --nocapture` | none | static_dro_baseline | Full | dro.v1 | none |

### 1.4 Registry Rows — Crash/Fault (CRF) — Provisional

| claim_id | evidence_id | owner | rerun_entrypoint | oracle | quick | notes |
|----------|-------------|-------|------------------|--------|-------|-------|
| bd-db300.7.2 | CRF-01:fsqlite_mvcc:baseline_unpinned:c1:42 | crash-runner | `realdb-e2e verify-suite --scenario CRF-01 --depth full` | wal_invariant | Full | **Blocked: G2 injection framework OPEN** |
| bd-db300.7.2 | CRF-02:fsqlite_mvcc:baseline_unpinned:c2:42 | crash-runner | `realdb-e2e verify-suite --scenario CRF-02 --depth full` | wal_invariant | Full | **Blocked: G2 injection framework OPEN** |

### 1.5 Registry Rows — Topology-Stress (TOP)

| claim_id | evidence_id | owner | rerun_entrypoint | oracle | quick |
|----------|-------------|-------|------------------|--------|-------|
| bd-db300.5.6.1 | TOP-01:fsqlite_mvcc:adversarial_cross_node:c8:42 | topo-runner | `realdb-e2e verify-suite --scenario TOP-01 --depth full --placement adversarial_cross_node` | self_consistency | Full |
| bd-db300.5.6.1 | TOP-02:fsqlite_mvcc:adversarial_cross_node:c8:42 | topo-runner | `realdb-e2e verify-suite --scenario TOP-02 --depth full --placement adversarial_cross_node` | self_consistency | Full |

### 1.6 Registry Rows — Rollback/Recovery (RBR)

| claim_id | evidence_id | owner | rerun_entrypoint | oracle | quick |
|----------|-------------|-------|------------------|--------|-------|
| bd-db300.9.12 | MVCC-RBR-01:fsqlite_mvcc:baseline_unpinned:c4:42 | suite-runner | `realdb-e2e verify-suite --scenario MVCC-RBR-01 --depth full` | rusqlite | Full |
| bd-db300.9.12 | MVCC-RBR-02:fsqlite_mvcc:baseline_unpinned:c4:42 | suite-runner | `realdb-e2e verify-suite --scenario MVCC-RBR-02 --depth full` | rusqlite | Full |

---

## 2. Artifact-Graph Linkage

When a verification row involves adaptive or budgeted behavior (E4 guardrail,
DRO abort controller, GC escalation), the evidence bundle must carry these
additional traceability fields:

```
manifest.json:
  "claim_id": "bd-db300.5.4.3",          ← What scorecard cell or bead this proves
  "evidence_id": "MVCC-PFA-04:...",      ← Unique evidence key
  "trace_id": "uuid-v4",                 ← Links to all structured log events in this run
  "policy_id": "e4.3.v1",               ← Version of the guardrail/controller policy active
  "decision_id": "trace_id:42",          ← Specific decision record in the guardrail log
  "budget_id": "lane_0_writer",          ← Which lane budget was exercised
  "slo_id": "p99_commit_10ms",           ← Which SLO was being guarded
  "shadow_lineage": "run_id_conservative", ← If shadow mode was active, the baseline run
  "fallback_lineage": "none | safe_mode"   ← Whether fallback/safe-mode was active
```

**Tracing join:** Given a `claim_id` (e.g., "c4 disjoint throughput improved
by 20%"), an operator queries:

```
1. claim_id → evidence_id (this registry)
2. evidence_id → artifact_root (manifest.json)
3. artifact_root → logs/run.jsonl (structured log)
4. grep trace_id in run.jsonl → all events for this run
5. filter by policy_id → guardrail/controller decisions
6. filter by decision_id → specific decision that affected the claim
```

No step requires reading a planning doc. The chain is
claim → evidence → artifact → log → decision.

---

## 3. Compatibility Selectors

Each code-changing bead touches one or more compatibility surfaces. The
selector determines which oracle and comparison method apply.

| Compatibility Surface | Oracle | Comparison Method | Allowed Differences |
|----------------------|--------|-------------------|-------------------|
| **row_level** | rusqlite | Per-row value comparison on deterministic workload | none (exact match) |
| **integrity_check** | PRAGMA integrity_check | String comparison of output | none (must be "ok") |
| **hot_path_profile** | Previous commit's profile | Numeric delta with threshold | ±5% throughput, ±10% p99 |
| **topology_metrics** | Self-consistency | CommitIndex monotonicity, lock exclusivity | none (invariants must hold) |
| **wal_invariant** | WAL spec (frame checksums, monotonic sequence) | Post-recovery database state | none |
| **ssi_serializable** | SSI cycle detection (Cahill/Fekete) | No committed write-skew anomaly | none |

### Selector Assignment by Bead Family

| Bead Family | Primary Surface | Secondary Surface |
|-------------|----------------|-------------------|
| WS1 (fixed-cost) | hot_path_profile | row_level |
| WS2 (contention geometry) | hot_path_profile, topology_metrics | row_level |
| WS3 (version chain) | row_level, integrity_check | hot_path_profile |
| WS4 (abort policy) | ssi_serializable | hot_path_profile |
| D1 (parallel WAL) | wal_invariant, integrity_check | row_level |
| E2 (fused entry) | row_level | hot_path_profile |
| E3 (metadata publication) | row_level, topology_metrics | — |

---

## 4. Gap-Conversion Rules

When a verification, compatibility, or evidence-linkage obligation is missing,
the following rules determine what happens:

### R1: Missing Evidence Bundle → New Bead

```
IF a claim_id references an evidence_id that has no artifact bundle:
  → Create a child bead under the claim_id's parent epic
  → Title: "Produce evidence for {claim_id}: {evidence_id}"
  → Priority: same as claim_id
  → The claim CANNOT appear on the scorecard until the evidence bead is closed
```

### R2: Missing Rerun Entrypoint → Block

```
IF an evidence_id has no rerun_entrypoint:
  → The evidence is not reproducible
  → Mark the registry row with status=BLOCKED
  → Create a bead to add the entrypoint
  → No claim may cite this evidence until reproducible
```

### R3: Missing Oracle → Explicit Accept

```
IF a verification class requires an oracle but none is assigned:
  → The row must have oracle=none AND an explicit justification
  → Self-consistency checks (CommitIndex monotonicity, lock exclusivity)
    are acceptable for TOP class
  → For COR/RBR/CRF, missing oracle is always a gap → new bead
```

### R4: Compatibility Surface Drift → Revalidation Bead

```
IF a code change touches a file covered by a compatibility surface
AND no passing evidence exists at the current commit:
  → The compatibility obligation is unsatisfied
  → Create a revalidation bead: "Rerun {evidence_id} after {commit_sha}"
  → Priority: P1 (must pass before next scorecard snapshot)
```

### R5: Policy Artifact Missing → Block Decision-Plane Claims

```
IF a claim involves adaptive behavior (E4 guardrail, DRO, GC escalation)
AND the evidence bundle lacks policy_id or decision_id:
  → The claim is not auditable
  → Create a bead: "Add policy traceability to {evidence_id}"
  → Decision-plane claims cannot appear on the scorecard until traceable
```

### R6: Shadow-Run Divergence → Investigate Before Claiming

```
IF shadow_lineage is non-null AND shadow_verdict = Diverged:
  → The aggressive mode produced different results from the conservative baseline
  → Create a bead: "Investigate shadow divergence in {evidence_id}"
  → The aggressive mode CANNOT become the default until divergence is resolved
```

---

## 5. Operator Manifest Surface

### 5.1 Rendering the Full Registry

```bash
scripts/verify_g5_3_rerun_manifest.sh
```

This script:
1. Reads the scenario registry (static array or JSON manifest).
2. For each row, verifies:
   - The rerun_entrypoint command is syntactically valid.
   - The oracle is assigned (or explicitly justified for TOP).
   - The claim_id maps to an existing bead.
   - The compatibility_surface is in the known set.
3. Reports coverage:
   - Number of Quick-mandatory rows with evidence.
   - Number of Full rows with evidence.
   - Number of rows with status=BLOCKED (and why).
4. Reports gaps:
   - Rows missing evidence bundles (R1 violations).
   - Rows missing rerun entrypoints (R2 violations).
   - Policy-bearing rows missing policy_id (R5 violations).
5. Emits `artifacts/g5_3_rerun_manifest.json` with the full registry.
6. Exits 0 if no R1/R2 violations in Quick-mandatory rows, 1 otherwise.

### 5.2 CI Integration

CI calls:
```bash
scripts/verify_g5_3_rerun_manifest.sh --quick-only
```

This variant only checks Quick-mandatory rows and exits 1 if any Quick
row has a gap. Full-suite gaps are reported as warnings.

### 5.3 Per-Claim Evidence Lookup

An operator or agent asking "what evidence supports claim X?" runs:

```bash
# Find all evidence rows for a specific claim
jq '.rows[] | select(.claim_id == "bd-db300.4.3.1")' artifacts/g5_3_rerun_manifest.json
```

Output:
```json
{
  "claim_id": "bd-db300.4.3.1",
  "evidence_id": "MVCC-PFA-01:fsqlite_mvcc:baseline_unpinned:c1:42",
  "rerun_entrypoint": "realdb-e2e hot-profile --workload commutative_inserts --concurrency 1",
  "oracle": "none",
  "compatibility_surface": "hot_path_profile",
  "status": "pass",
  "artifact_root": "artifacts/MVCC-PFA-01/20260323T080000/"
}
```

No planning docs need to be read. The claim → evidence → artifact chain
is fully mechanical.

---

## 6. Decision-Record Contract for Policy-Driven Behavior

When a verification row exercises a controller (E4 guardrail, DRO, placement),
the evidence bundle's `logs/run.jsonl` must contain decision records with at
minimum:

| Field | Type | Description |
|-------|------|-------------|
| `trace_id` | String | Links to manifest |
| `policy_id` | String | Controller version (`e4.3.v1`, `dro.v1`) |
| `decision_id` | String | Monotonic counter within this run |
| `top_evidence_terms` | String[] | Top 3 signals that drove the decision |
| `posterior_or_confidence` | f64 | Controller's confidence in the decision |
| `expected_loss` | f64 | Expected loss of the chosen action |
| `selected_action` | String | What the controller decided |
| `best_alternative_action` | String | What the counterfactual was |
| `regret_delta` | f64 | Estimated regret of the chosen action vs alternative |
| `fallback_active` | bool | Whether safe-mode or fallback was in effect |
| `fallback_reason` | String | Why fallback activated (if applicable) |
| `policy_artifact_version` | String | Exact version of the policy artifact |

This contract ensures that a regression in a decision-plane bead can be traced
to the exact decision that caused it, without re-deriving controller internals.

---

## 7. Shadow-Run and Baseline-Comparator Lineage

For any evidence row where the aggressive mode (DRO adaptive, E4 guardrail
non-default) is the run under test:

| Field | Content | Purpose |
|-------|---------|---------|
| `shadow_lineage` | Run ID of the conservative/baseline run | "What would have happened without the optimization?" |
| `shadow_verdict` | clean / diverged / not_run | Did the aggressive mode produce the same result? |
| `baseline_comparator` | `sqlite3_c1_baseline` or `previous_commit` | What the performance delta is computed against |

**Rule:** If `shadow_verdict = diverged`, the aggressive mode cannot be
promoted to default until a divergence investigation bead is created and
closed (gap-conversion rule R6).

---

## 8. Dependency Gap

| Blocking Bead | Status | Impact | Mitigation |
|--------------|--------|--------|------------|
| **bd-db300.7.5.2** (G5.2: crash/fault mapping) | OPEN | CRF registry rows are provisional. Fault injection points and crash timing details depend on G5.2's cross-epic mapping. | CRF rows are included with status=BLOCKED and note "pending G2 injection framework." Quick-suite coverage is not affected (CRF is Full-only). |

---

## 9. Consequences for Downstream

This bead unblocks 20+ downstream beads. Key consumers:

| Downstream | What It Gets |
|------------|-------------|
| **G6.1** (logging schema) | `trace_id`/`scenario_id`/`policy_id` join keys |
| **G6.3** (artifact manifests) | Registry row → manifest.json field mapping |
| **G7.1** (verify-suite entrypoints) | Rerun commands per scenario |
| **G5.5** (regime atlas) | Regime-to-evidence mapping |
| **G5.6** (shadow oracle) | Shadow lineage fields |
| **G6.4** (policy-as-data) | Decision-record contract fields |
| **D1.b–D1.e** (parallel WAL) | Evidence requirements for WAL claims |
| **E2.2.c–E2.2.d** (fused entry) | Evidence requirements for entry-path claims |
| **E3.3.a–E3.3.b** (metadata publication) | Evidence requirements for publication claims |
| **G4** (final scorecard) | Claim → evidence → artifact chain for every scorecard cell |
