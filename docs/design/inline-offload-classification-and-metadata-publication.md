# Inline vs Offloaded Work Classification & Metadata Publication Primitives

**Beads:** `bd-db300.5.4.1` (E4.1) + `bd-db300.5.3.1` (E3.1)
**Date:** 2026-03-22
**Status:** Design artifact — ready for downstream budgeting (E4.2) and primitive mapping (E3.2)
**Depends on:** E1 state-placement map, ADR-0002 (tiny-publish shared state), B3 (lock-free publication work)

---

## Purpose

This document delivers two tightly coupled artifacts:

1. **E4.1 — Inline vs offloaded work classification**: partition every operation
   on the writer commit path into one of four execution classes (INLINE-CRITICAL,
   INLINE-FAST, OFFLOAD-ASYNC, OFFLOAD-BACKGROUND), tied to latency ownership
   and queue-budgeting constraints.

2. **E3.1 — Metadata class publication map**: partition every reader-visible
   metadata surface by read-write ratio, retryability, snapshot semantics, and
   reclamation difficulty, so E3.2 can select publication primitives without
   reopening architectural questions.

These two classifications are coupled because the publication primitive chosen
for a metadata class constrains whether the corresponding publication work
must be inline or can be offloaded — and conversely, a work-class decision
constrains which publication primitives are available.

---

## Grounding

### Architecture: Tiny-Publish Shared State (ADR-0002)

The selected Track E architecture keeps Stages 1–3 lane-local, makes Stage 4
(first-touch arbitration) narrow, concentrates all irreducible global work into
a tiny Stage 5 publish window, and pushes everything else to asynchronous
Stage 6. This classification must not widen Stages 4 or 5.

### Evidence: Live Pipeline (E1 State Placement)

The live pipeline stages, ownership boundaries, and shared touch points are
documented in `docs/design/many-core-transaction-pipeline-state-placement.md`.
The invariants INV-DB300-E1.1-1 through E1.1-8 apply throughout.

### Evidence: Measured Performance Facts

From the performance program (2026-03-21 snapshot):
- c1 disjoint: 0.205x (dominated by fixed-cost overhead, not contention)
- c4 disjoint: 0.553x (contention tax prevents inheriting c8 geometry)
- c8 disjoint: 3.140x (enough parallelism to amortize shared surfaces)
- c4 mixed: 0.963x (near parity, publication overhead starting to show)
- c8 mixed: 5.903x (publication cost amortized by parallelism)

The c4 gap relative to c8 is the primary target. Work that runs inline at c4
but could be offloaded represents the highest-EV scheduling lever.

---

## Part 1: Inline vs Offloaded Work Classification (E4.1)

### Execution Classes

| Class | Abbreviation | Contract | Latency Budget |
|-------|-------------|----------|----------------|
| **INLINE-CRITICAL** | IC | Must complete before commit is visible. Failure aborts the transaction. Runs on writer thread inside the publish window. | ≤ 500ns per page at p99 |
| **INLINE-FAST** | IF | Must complete before control returns to the application. May run after publish but before `COMMIT` returns. Failure is logged, not fatal. | ≤ 5μs total at p99 |
| **OFFLOAD-ASYNC** | OA | May complete after `COMMIT` returns. Must complete before the next checkpoint or visibility-critical event. Failure triggers retry or degraded-mode fallback. | ≤ 100μs, amortized |
| **OFFLOAD-BACKGROUND** | OB | Best-effort. May be batched, delayed, or dropped under load. Failure is observable but not fatal. | Unbounded, but budgeted queue depth |

### Classification Table

Each row is a concrete operation on the writer commit path, classified by
execution class, the shared surface it touches, and the invariant it serves.

#### Stage 2: Transaction Admission

| Operation | Class | Shared Surface | Invariant | Rationale |
|-----------|-------|---------------|-----------|-----------|
| Bind pager publication snapshot | IC | PagerPublishedSnapshot | INV-E1.1-2 (snapshot validity) | Snapshot must be bound before any commit can advance past it. Cannot be deferred. |
| Register concurrent session in ConcurrentRegistry | IF | ConcurrentRegistry Mutex | INV-E1.1-1 (concurrent-by-default) | Registration must complete before first statement executes. Not critical-path for other writers. |

#### Stage 3: Statement Execution

| Operation | Class | Shared Surface | Invariant | Rationale |
|-----------|-------|---------------|-----------|-----------|
| Already-owned page writes | — (lane-local) | None | — | No shared interaction. Pure local work. |
| Read from CommitIndex for visibility check | IC | CommitIndex.fast_array (Acquire load) | INV-E1.1-4 (monotonicity) | Single atomic load per page. Already O(1), no lock. |

#### Stage 4: First-Touch Arbitration

| Operation | Class | Shared Surface | Invariant | Rationale |
|-----------|-------|---------------|-----------|-----------|
| Page lock acquisition (CAS on fast array) | IC | InProcessPageLockTable fast_array | INV-E1.1-3 (first-touch exclusivity) | Single CAS instruction. Must succeed or fail before the page can be written. |
| Page lock acquisition (sharded fallback, page > 65536) | IC | InProcessPageLockTable shard Mutex | INV-E1.1-3 | Rare path. Shard Mutex held briefly. Still inline because ownership must be resolved before write proceeds. |
| CommitIndex staleness check on first touch | IC | CommitIndex.fast_array (Acquire load) | INV-E1.1-4 | FCW precondition: must verify no concurrent commit advanced this page since snapshot. |
| Page-1 structural conflict tracking | IC | Synthetic conflict surface | INV-E1.1-6 (structural explicitness) | Rare path. Must be resolved before commit planning because it widens the conflict surface. |
| Wait-for-page-lock-holder-change (parking) | IC | Lock table waiter queue | INV-E1.1-3 | Writer is blocked. Cannot proceed until lock is available. Not offloadable — the writer IS the waiter. |

#### Stage 5: Commit Planning, Durable Commit, and Publish

| Operation | Class | Shared Surface | Invariant | Rationale |
|-----------|-------|---------------|-----------|-----------|
| SSI validation (FCW check against CommitIndex) | IC | CommitIndex (reads), ConcurrentRegistry (Mutex) | INV-E1.1-4, INV-E1.1-5 | Must determine commit/abort before durable order is assigned. |
| Pager txn.commit() — WAL frame write | IC | Pager internal state, WAL file | INV-E1.1-5 (publish-after-durable) | Durable write is irreducibly on the critical path. The write is already committed to durable storage when this returns. |
| advance_commit_clock() — atomic fetch_add on next_commit_seq | IC | next_commit_seq (AtomicU64 AcqRel) | INV-E1.1-5 | Irreducibly global. Single instruction. The durable-order allocator. |
| CommitIndex.batch_update() — publish visibility | IC | CommitIndex.fast_array (Relaxed stores after Release fence) | INV-E1.1-4, INV-E1.1-5 | Must complete before lock release so readers see consistent state. |
| Page lock release (CAS on fast array) | IC | InProcessPageLockTable fast_array | INV-E1.1-3 | Must follow visibility publication. Releases ownership to waiting writers. |
| Lock-release waiter wakeup | IF | Lock table waiter queue | — | Wakeup can be slightly delayed. Waiters are parking, not spinning. Moving this after the critical section reduces publish-window duration. |
| SSI abort decision + evidence snapshot | IC (abort path only) | ConcurrentRegistry | — | Only on abort. Must complete before returning BUSY to caller. |
| SSI commit evidence card recording | OA | Evidence ledger | — | Already async via `record_async()`. Observable audit trail, not correctness-critical. |

#### Stage 6: Post-Commit Cleanup

| Operation | Class | Shared Surface | Invariant | Rationale |
|-----------|-------|---------------|-----------|-----------|
| Active txn handle drop + RefCell cleanup | IF | None (lane-local) | — | Must complete before next statement begins. Trivial cost. |
| VTab commit notification | OA | Extension vtab state | — | Best-effort. Extensions should not block the commit return. |
| MemDatabase staleness update | IF | memdb_visible_commit_seq (RefCell, lane-local) | — | Lane-local update. Trivial. |
| Differential commit invalidation emission | OA | Invalidation channel | — | Readers will pick up invalidation on next snapshot bind. Does not need to be synchronous. |
| Time-travel snapshot capture | OB | Time-travel ring buffer (lane-local) | — | Diagnostic/debug feature. May be expensive (clones entire MemDatabase). |
| Adaptive autocheckpoint | OB | Pager checkpoint state | INV-E1.1-7 (reclamation) | Already a separate concern. Should never block commit return. Must respect WAL size bounds eventually, but not synchronously. |
| MVCC GC tick | OB | VersionStore, VersionGuardRegistry | INV-E1.1-7 | Reclamation is correctness work but not on the commit critical path. Must respect active snapshot horizon. |
| ConcurrentRegistry session recycle | IF | ConcurrentRegistry Mutex | — | Handle recycle is fast (remove from HashMap, push to free list). Slightly delays Mutex release but avoids a second lock acquisition. |

### Publish-Window Composition

The INLINE-CRITICAL publish window (Stage 5, from SSI validation through lock
release) currently contains:

```
┌─────────────────── PUBLISH WINDOW ───────────────────┐
│                                                       │
│  SSI validation (reads: CommitIndex, registry)        │  IC
│  pager txn.commit() (WAL write)                       │  IC — DOMINATES
│  advance_commit_clock (1 atomic fetch_add)            │  IC — ~1ns
│  CommitIndex.batch_update (1 fence + N relaxed stores)│  IC — ~5ns/page
│  page lock release (N CAS operations)                 │  IC — ~5ns/page
│                                                       │
└───────────────────────────────────────────────────────┘
  THEN (outside window):
    waiter wakeup                                          IF
    session recycle                                        IF
    evidence recording                                     OA
    snapshot capture, GC, checkpoint                        OB
```

**Key observation:** The pager `txn.commit()` (WAL frame write) dominates the
publish window when I/O is involved (file-backed DBs). For `:memory:` databases,
the atomic operations and CommitIndex updates dominate. The c4→c8 scaling gap is
most likely caused by contention on `ConcurrentRegistry` Mutex during SSI
validation, not by the atomic operations themselves.

### Inline→Offload Promotion Candidates

Operations that are currently INLINE but could be promoted to OFFLOAD with
careful design:

| Operation | Current | Target | Precondition | Risk |
|-----------|---------|--------|--------------|------|
| Waiter wakeup | IC (inside lock release) | IF (after publish window) | Lock release CAS is separate from wakeup dispatch | Low: waiters are already parking, 1–5μs delay is invisible |
| SSI evidence recording | OA (already) | OA | Already correct | None |
| VTab notification | IF | OA | VTab state must tolerate delayed notification | Low: extensions already handle async |
| Checkpoint | OB (already) | OB | Already correct | None |
| GC tick | OB (already) | OB | Already correct | None |

Operations that MUST NOT be promoted:

| Operation | Why |
|-----------|-----|
| SSI validation | Determines commit/abort. Cannot defer. |
| Pager WAL write | Durability contract. Cannot skip. |
| Commit clock advance | Durable order assignment. Irreducibly global. |
| CommitIndex publication | Readers must see committed state before lock release. |
| Lock release | Must follow publication. Waiters depend on it. |

### Fallback Behavior

If the scheduling system cannot classify an operation (e.g., a new path is added
without classification), the safe fallback is:

- **Default: INLINE-FAST.** This is correct (never delays visibility) and
  observable (adds latency to commit return, which will show in benchmarks).
- **Never default to OFFLOAD.** An incorrectly offloaded operation can violate
  visibility or ordering invariants silently.

---

## Part 2: Metadata Class Publication Map (E3.1)

### Classification Axes

Each metadata class is scored on five axes:

| Axis | Description | Scale |
|------|-------------|-------|
| **Read:Write ratio** | How many readers per write | R:W ratio (higher = more read-heavy) |
| **Retryability** | Can a reader retry a stale/torn read? | Yes / Conditional / No |
| **Snapshot semantics** | Must readers see a consistent point-in-time snapshot? | Strict / Relaxed / None |
| **Reclamation** | Does old metadata need explicit lifecycle management? | EBR / Grace / Immediate / None |
| **Topology sensitivity** | Does cross-NUMA access to this metadata materially hurt? | High / Medium / Low |

### Metadata Classes

#### Class M1: Per-Page Commit Visibility (CommitIndex)

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~100:1 to ~1000:1 | Every first-touch read checks CommitIndex; writes happen only at commit |
| Retryability | **Yes** | Readers can retry: a stale read just means the reader doesn't see the latest commit, which is correct under snapshot semantics |
| Snapshot semantics | **Relaxed** | Monotonicity is required (INV-E1.1-4), but readers don't need a consistent cross-page snapshot of the CommitIndex — they read per-page |
| Reclamation | **None** | CommitIndex entries are overwritten in place (atomic store). No old-version lifecycle. |
| Topology sensitivity | **High** | Hot pages cause cache-line bouncing between writer (store) and readers (load) across NUMA nodes |

**Current primitive:** Tier-1 flat AtomicU64 array (pages ≤ 65536) + Tier-2 sharded LeftRight (pages > 65536).

**Publication constraint:** Writer must issue Release fence before stores, reader must issue Acquire load. Stores are Relaxed after the fence. This is already optimal for the single-writer-many-reader pattern per page.

**Observation for E3.2:** The flat array is already near-optimal. The LeftRight tier for large pages adds unnecessary Mutex overhead. A sharded epoch-reclaimed concurrent hash map or extending the flat array to 262144 entries (2 MiB) would eliminate the slow tier.

---

#### Class M2: Page Ownership Directory (InProcessPageLockTable)

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~1:1 | Each first-touch acquires a lock (write); each commit releases it (write). Reads happen during FCW validation. |
| Retryability | **No** | Lock acquisition is a CAS — it either succeeds or the writer must wait/abort. Not retryable in the seqlock sense. |
| Snapshot semantics | **Strict** | Lock state must be exact: either this txn owns the page or it doesn't. Stale reads cause lost-update bugs. |
| Reclamation | **None** | Locks are released in place (atomic store to 0). No version chain. |
| Topology sensitivity | **High** | Contended CAS on the same cache line between writers on different NUMA nodes is the worst-case cross-node penalty. |

**Current primitive:** Tier-1 flat AtomicU64 array (CAS) + Tier-2 sharded Mutex<HashMap>.

**Publication constraint:** CAS is inherently linearizable. No weaker primitive is safe here. The only optimization axis is reducing contention (via conflict-geometry routing, E5) or reducing the number of first-touch events (via page-affinity hints).

**Observation for E3.2:** This is already at the correct primitive. The E5 (conflict-topology routing) and E6 (lane placement) beads are the right levers, not a publication primitive change.

---

#### Class M3: Global Commit Sequence Counter (next_commit_seq)

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~0:1 | Written once per commit. Readers don't read this directly — they read CommitIndex (M1) or snapshot bindings (M5). |
| Retryability | **N/A** | Not read in the hot path. |
| Snapshot semantics | **Strict** | Commit ordering is a total order. Must be monotonically increasing. |
| Reclamation | **None** | Single counter, overwritten atomically. |
| Topology sensitivity | **High** | fetch_add on a single cache line from multiple NUMA nodes is the textbook worst case. Already batched via CommitSequenceCombiner. |

**Current primitive:** AtomicU64 with AcqRel fetch_add, batched through CommitSequenceCombiner (flat combining).

**Publication constraint:** Total order is required. The combiner already reduces cache-line traffic by ~16x under load. Further reduction requires either HTM (bd-77l3t) or per-NUMA-node pre-allocation with global reconciliation.

**Observation for E3.2:** The combiner is the correct primitive. HTM fast-path (bd-77l3t) is the natural next optimization. Per-NUMA pre-allocation adds complexity with marginal gain given combiner batching already works.

---

#### Class M4: Concurrent Transaction Registry (ConcurrentRegistry)

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~1:2 | Reads during SSI validation (once per commit for each active txn). Writes: begin (1), plan (1), finalize (1), recycle (1). |
| Retryability | **No** | SSI validation must see the exact set of active transactions. A stale view causes false negatives (missed conflicts). |
| Snapshot semantics | **Strict** | The registry snapshot during SSI validation must be linearizable with respect to concurrent commits. |
| Reclamation | **Immediate** | Session handles are recycled on finalize. No deferred reclamation needed. |
| Topology sensitivity | **Medium** | Mutex is held briefly. Cross-NUMA penalty exists but is amortized by the (relatively) long SSI validation work inside the lock. |

**Current primitive:** Arc<Mutex<ConcurrentHandle>> per session, with a global Mutex on the registry HashMap.

**Publication constraint:** SSI validation must atomically observe: (a) which transactions are active, (b) their read/write sets, (c) their commit status. This is inherently a consistent-snapshot problem.

**Observation for E3.2:** The global Mutex is the most promising optimization target for c4. Options:
1. **Sharded registry** by session-id hash — reduces lock contention but complicates SSI scan.
2. **RCU-style snapshot** — writers publish via copy-on-write; readers see a consistent snapshot without blocking writers. Reclamation via epoch.
3. **Seqlock + per-session atomics** — readers retry if registry changed during scan. Works if SSI scan is fast relative to commit rate.

Recommendation: Option 2 (RCU snapshot) for E3.2, because SSI validation is read-dominant during the critical window, and the number of active sessions is small (bounded by core count).

---

#### Class M5: Pager Publication Snapshot

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~N:1 (N = concurrent readers per commit) | Written once per commit. Read by every transaction at BEGIN to bind snapshot. |
| Retryability | **Yes** | A reader that sees a slightly old snapshot just starts from an earlier consistent state. Safe under MVCC. |
| Snapshot semantics | **Relaxed** | Must be internally consistent (commit_seq matches page_set_size), but a reader seeing the previous snapshot is correct. |
| Reclamation | **None** | Overwritten in place. |
| Topology sensitivity | **Medium** | Read on BEGIN, not on every page access. Amortized over transaction lifetime. |

**Current primitive:** Mixed atomics + RefCell behind pager Mutex.

**Publication constraint:** Internal consistency required (multi-field snapshot). Relaxed freshness is acceptable.

**Observation for E3.2:** SeqLockPair is the ideal primitive. The codebase already has SeqLock and SeqLockPair implementations. The pager publication snapshot should be migrated to SeqLockPair to eliminate Mutex contention on the read path during BEGIN.

---

#### Class M6: Schema/Pragma Metadata

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~10000:1 | Schema changes are rare (DDL). Reads happen on every statement for schema validation. |
| Retryability | **Yes** | A reader seeing a slightly stale schema will recompile the statement, which is correct (schema epoch check). |
| Snapshot semantics | **Relaxed** | Schema epoch is sufficient for staleness detection. No cross-field consistency needed. |
| Reclamation | **None** | Schema data is owned by connection-local registries. Global publication is epoch-only. |
| Topology sensitivity | **Low** | Single atomic read per statement (schema epoch check). Extremely low traffic. |

**Current primitive:** SeqLock for schema_epoch.

**Publication constraint:** Already optimal. SeqLock gives sub-nanosecond reads with no writer blocking.

**Observation for E3.2:** No change needed. This is already at the correct primitive.

---

#### Class M7: Version Store (Committed Page Versions)

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~5:1 to ~50:1 | Written once per page per commit. Read by every transaction that needs a historical page version. |
| Retryability | **No** | A reader traversing the version chain must see consistent versions. Torn reads cause data corruption. |
| Snapshot semantics | **Strict** | Version chains must be consistent with CommitIndex visibility. A reader must never see a partial version entry. |
| Reclamation | **EBR** | Old versions must persist until no active snapshot can see them. Epoch-based reclamation (crossbeam-epoch) is the correct discipline. |
| Topology sensitivity | **Medium** | Version chain traversal is pointer-chasing, which is inherently NUMA-sensitive. But this is read-only for committed versions. |

**Current primitive:** Shared concurrent structure with epoch-based GC via gc_tick.

**Publication constraint:** Append-only for committed versions. GC is the only mutating operation on historical entries. The publication primitive must guarantee that a newly committed version is visible before the corresponding CommitIndex entry is published.

**Observation for E3.2:** The current structure is correct but should ensure that version store insertion happens-before CommitIndex publication (which it does: `finalize_prepared_concurrent_commit_with_ssi` publishes versions before CommitIndex batch_update). The GC path (bd-wnk1r, bd-bolsv) is the optimization target, not the publication primitive.

---

#### Class M8: SSI Evidence and Observability

| Axis | Value | Evidence |
|------|-------|----------|
| Read:Write | ~0.01:1 | Written on every commit/abort. Read only by diagnostic tools and the abort-rate controller. |
| Retryability | **Yes** | Evidence is informational. Stale reads cause suboptimal policy decisions, not correctness violations. |
| Snapshot semantics | **None** | Best-effort. Loss of individual evidence cards is acceptable. |
| Reclamation | **None** | Ring buffer or append log with bounded size. Oldest entries are overwritten. |
| Topology sensitivity | **Low** | Written once, read rarely. No hot cache-line contention. |

**Current primitive:** Async recording via `record_async()`.

**Publication constraint:** Already correct. Fire-and-forget is the right model. The abort-rate controller (bd-3t52f) will read this asynchronously.

**Observation for E3.2:** No change needed.

---

### Metadata Class Summary Matrix

| Class | Surface | R:W | Retry | Snapshot | Reclaim | NUMA | Current Primitive | Candidate Upgrade |
|-------|---------|-----|-------|----------|---------|------|-------------------|-------------------|
| M1 | CommitIndex | 100:1+ | Yes | Relaxed | None | High | AtomicU64 array + LeftRight | Extend flat array; replace LeftRight with epoch hash |
| M2 | PageLockTable | 1:1 | No | Strict | None | High | AtomicU64 array + shard Mutex | No primitive change; routing (E5) is the lever |
| M3 | next_commit_seq | 0:1 | N/A | Strict | None | High | AtomicU64 + CommitSequenceCombiner | HTM fast-path (bd-77l3t) |
| M4 | ConcurrentRegistry | 1:2 | No | Strict | Immediate | Medium | Arc<Mutex<HashMap>> | **RCU snapshot for SSI reads** |
| M5 | PagerPublishedSnapshot | N:1 | Yes | Relaxed | None | Medium | Mutex + atomics | **SeqLockPair** |
| M6 | Schema/Pragma epoch | 10000:1 | Yes | Relaxed | None | Low | SeqLock | No change |
| M7 | VersionStore | 5:1+ | No | Strict | EBR | Medium | Epoch-based concurrent store | GC optimization (WS3) |
| M8 | SSI Evidence | 0.01:1 | Yes | None | None | Low | Async ring buffer | No change |

---

## Cross-Reference: Work Classification × Metadata Class

This matrix shows which metadata classes are touched by each inline/offload work
class, establishing the coupling between Parts 1 and 2:

| Metadata Class | IC Operations | IF Operations | OA Operations | OB Operations |
|----------------|---------------|---------------|---------------|---------------|
| M1 CommitIndex | First-touch staleness check, batch_update publish | — | — | — |
| M2 PageLockTable | Lock acquire (CAS), lock release (CAS) | Waiter wakeup | — | — |
| M3 next_commit_seq | advance_commit_clock (fetch_add) | — | — | — |
| M4 ConcurrentRegistry | SSI validation (read, under Mutex) | Session recycle | — | — |
| M5 PagerPublishedSnapshot | Snapshot bind (read) | — | — | — |
| M6 Schema epoch | — | — | — | — (read-only on statement path) |
| M7 VersionStore | — | — | — | GC tick |
| M8 SSI Evidence | — | — | Evidence recording | — |

**Key insight:** The IC (INLINE-CRITICAL) column shows exactly what must be in
the publish window. M1, M2, M3, and M4 are the only metadata classes touched
inline-critical. Of these, M4 (ConcurrentRegistry) is the most promising
optimization target because it is the only one still using a coarse Mutex where
a finer primitive is viable.

---

## Queueing-Theoretic Framing for E4.2

This section sets up the budgeting inputs for the downstream queue-depth and
helper-lane bead (bd-db300.5.4.2).

### Little's Law Application

For the publish window:

```
L = λ · W

where:
  L = average number of writers in the publish window simultaneously
  λ = commit arrival rate (commits/second)
  W = average time in the publish window (seconds)
```

**Measured estimates (from c4 benchmark context):**
- λ ≈ 40,000 commits/sec (c4 disjoint, target)
- W ≈ 1μs (IC operations only, in-memory DB)
- W ≈ 50μs (IC operations, file-backed with WAL fsync)
- Therefore L_memory ≈ 0.04 (publish window is not the bottleneck for in-memory)
- Therefore L_file ≈ 2.0 (at c4, ~2 writers in publish window simultaneously)

For file-backed DBs, the WAL write dominates W. Group commit (batching WAL
writes across concurrent writers) is the primary lever to reduce effective W.

**Backpressure trigger:**
When L > c (where c = number of writers that can physically fit in the publish
window without serialization), additional arrivals must be backpressured at
Stage 4 (first-touch arbitration) rather than allowed to queue at Stage 5.

### Helper-Lane Budget Classes

| Lane | Purpose | Queue Depth Bound | Wake-to-Run Budget | Starvation Policy |
|------|---------|-------------------|--------------------|-------------------|
| **Writer lane** (per-core) | Stages 1–5 IC/IF work | No queue — writer IS the lane | N/A | Fairness via page-lock wait ordering |
| **Wakeup lane** (per-NUMA or shared) | Waiter wakeup dispatch after lock release | Bounded: 2 × max_concurrent_writers | ≤ 10μs | Drain on checkpoint or idle |
| **Evidence lane** (shared) | SSI evidence recording, observability | Bounded: 64 entries | ≤ 1ms | Drop oldest on overflow |
| **GC lane** (shared) | Version chain reclamation | Bounded: 1 outstanding GC sweep | ≤ 100ms | Trigger on chain-depth threshold |
| **Checkpoint lane** (shared) | WAL checkpoint | Bounded: 1 outstanding checkpoint | ≤ 1s | Trigger on WAL size threshold |

### Admission Control Trigger

If the publish window occupancy L exceeds the backpressure threshold:

1. **First response:** Delay new transaction BEGIN by parking in admission queue.
2. **Second response:** If admission queue depth > 2 × core_count, reject new
   transactions with SQLITE_BUSY.
3. **Fallback:** Under sustained overload, checkpoint lane drains WAL to reduce
   per-commit W, which reduces L.

This ensures p50 latency is protected: ordinary commits never wait behind
a backlog of deferred work.

---

## Verification Plan

### Unit Tests Required

1. **IC classification correctness:** Instrument the publish window with
   timestamps. Assert that no OA or OB operation executes between SSI validation
   start and lock release completion.

2. **Offload safety:** For each OA/OB operation, test that commit is visible to
   readers BEFORE the offloaded operation completes. This proves the offload
   does not violate INV-E1.1-5.

3. **Metadata monotonicity:** For M1 (CommitIndex), property-test that
   batch_update never publishes a sequence number lower than the existing value
   for any page.

### E2E Scenarios Required

1. **c4 publish-window measurement:** Run the c4 disjoint benchmark with
   publish-window timing instrumentation. Capture p50/p95/p99 of W.

2. **Registry contention measurement:** Run the c4 mixed benchmark with
   ConcurrentRegistry lock-hold timing. Capture contention rate and p99 wait.

3. **Offload queue depth monitoring:** Run sustained c8 workload and verify
   that OA/OB queues stay within their depth bounds.

### Logging Artifacts Required

- `tracing::debug!(target: "fsqlite::commit::classify", op = %name, class = %class, duration_ns = %dur)`
- `tracing::info!(target: "fsqlite::commit::publish_window", writers_in_window = %l, window_duration_ns = %w)`
- `tracing::warn!(target: "fsqlite::commit::backpressure", trigger = %reason, queue_depth = %depth)`

---

## Assumptions Ledger

| ID | Assumption | Verification Method | Failure Mode |
|----|-----------|---------------------|--------------|
| A1 | WAL write dominates publish window for file-backed DBs | Profile txn.commit() duration breakdown | If false, atomic operations dominate — focus on combiner optimization |
| A2 | ConcurrentRegistry Mutex is the primary c4 contention source | Lock-hold timing + contention rate measurement | If false, CommitIndex or PageLockTable contention dominates — change E3.2 priority |
| A3 | Waiter wakeup can be safely moved outside the publish window | Test: writer releases lock, reader sees committed state before wakeup fires | If false, some reader depends on synchronous wakeup — keep wakeup in IC |
| A4 | RCU-style snapshot is viable for ConcurrentRegistry at ≤ 64 concurrent sessions | Prototype + benchmark at c4/c8 | If false, session count exceeds RCU copy budget — fall back to sharded Mutex |
| A5 | SeqLockPair eliminates Mutex contention on pager snapshot bind | Before/after c4 BEGIN latency measurement | If false, other BEGIN work dominates — deprioritize M5 change |

---

## Consequences for Downstream Beads

| Downstream Bead | What This Artifact Provides |
|-----------------|---------------------------|
| **bd-db300.5.4.2** (E4.2: queue budgets) | Lane definitions, queue depth bounds, Little's Law parameters, backpressure trigger formula |
| **bd-db300.5.3.2** (E3.2: primitive mapping) | Metadata class axes, current primitives, candidate upgrades, coupling constraints |
| **bd-db300.5.4.3** (E4.3: admission control) | Admission control trigger sequence, p50 protection mechanism |
| **bd-77l3t** (HTM fast-path) | M3 classification confirms CommitSequenceCombiner is the HTM target |
| **bd-3t52f** (DRO abort policy) | M8 classification confirms evidence is OA/fire-and-forget, safe for policy input |
| **bd-wnk1r / bd-bolsv** (version chain GC) | M7 classification confirms EBR discipline, GC is OB class |
