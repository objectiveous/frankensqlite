# Deadlock / Concurrency Audit — fsqlite-mvcc + fsqlite-pager (2026-04-24, cc_4)

Applied `/deadlock-finder-and-fixer` to `crates/fsqlite-mvcc` and
`crates/fsqlite-pager`. Every finding survived the skill's
false-positive validation checklist: **construct a concrete
interleaving**, **check `&mut self` ownership**, **trace the actual
call chain**, **measure critical-section duration**, **recognize
correct condvar / Relaxed patterns**.

## Class-by-class result

### Class 1 — Classic Mutex Deadlock (nested lock ordering)

**cell_visibility.rs** — `{txn_tracker, arena, heads}` three-lock set.
Enumerated every function that nests two or more:

| Function | Order observed |
|---|---|
| `record_delta` | HEADS(read, dropped) → TRACKER → ARENA(scoped inside) → HEADS(write, after drop) |
| `commit_txn` | TRACKER → ARENA |
| `rollback_txn` | TRACKER → ARENA → HEADS |
| `check_conflict` | TRACKER → ARENA |
| `gc` / `collect_visible_deltas` / `page_delta_count` / `clear_page_deltas` | ARENA → HEADS |
| `resolve` | HEADS(dropped) → ARENA (sequential, not nested) |

**Global order is consistent: `tracker ⟶ arena ⟶ heads`.** No function
ever takes them in a reverse direction. **CLEAN** — per the skill's
false-positive rule, "Consistent lock ordering across all sites is not
a 'risk' — it's a proof of safety."

**pager.rs:6761 `commit_wal_group_commit_with_snapshot`** — flagged as
"12 guards" by the static scan, but all 5 acquisitions are on the
*same* `queue.consolidator` mutex, each in a distinct scoped block
(lines 6843–6849, 6890–…, 6922–…, 6963–…, 6988–…). No nested
self-acquisition, no multi-lock cycle. **CLEAN.**

**pager.rs:8225 `SimpleTransaction::commit`** — all `self.inner.lock()`
acquisitions are scoped and release before re-acquisition. Phase
A/B/C commit split is the intent (bd-wee9a comment in-line). **CLEAN**
for deadlock; contention separately tracked in bd-wee9a.

**pager.rs:4658 `set_wal_backend`** — `inner.lock()` dropped at 4666
*before* `wal_backend.write()` acquired at 4670. Sequential, not
nested. **CLEAN.**

### Class 2 — Async / `.await` Deadlocks

Applied the skill's highest-ROI recipe across both crates:

```bash
rg --type rust -U 'let\s+\w+\s*=\s*[^;]*\.(lock|read|write)\(\)[^;]*;[^}]*\.await' \
  crates/fsqlite-mvcc crates/fsqlite-pager
```

**Zero matches. Zero `async fn` declarations.** Both crates are
synchronous. Class 2 is **N/A**.

### Class 4 — Database Concurrency

Not the target layer — these crates *implement* the lock/wait
primitives that the Class 4 patterns cover, rather than consuming
them. Relevant hazards (page-lock waiter ordering, chain-head CAS
stampede) were audited in prior sessions and have their own beads and
tests (e.g. `loom_chain_head_publication_linearizable` in
`invariants.rs`).

### Class 5 — LD_PRELOAD / Reentrant Init

Neither crate has `crate-type = cdylib`, `#[no_mangle] extern "C"`
exports, nor any code path reachable from the dynamic linker or a
signal handler. `OnceLock` / `OnceCell` / `Lazy` uses in both crates
are application-internal (user-initiated via `Connection::open`).
Per the skill's scope check: **N/A**.

### Class 8 — Poisoning

**pager.rs** uses `std::sync::Mutex` in several places and correctly
maps `.lock()` errors to `FrankenError::internal("... poisoned")`
(e.g. lines 4662, 4671, 8233, 8273). Poisoning is surfaced, not
silently ignored. **CLEAN.**

**mvcc crate** uses `fsqlite_types::sync_primitives::{Mutex, RwLock}`
— parking_lot shims that never poison by design. **CLEAN.**

### Class 9 — Memory Ordering

No new findings beyond those already handled in prior session commits
(branchless `visible` ec87700b, strong-CAS publish 4904047e,
candidate-free SSI 59250449, histogram/gauge gates bc4fa6b5 / f2707d1a
/ d2156302 / 03c49886).

## Single finding worth documenting

### FRAGILITY — `InProcessPageLockTable::drain_orphaned` callback-under-lock

- **Location:** `crates/fsqlite-mvcc/src/core_types.rs:1215-1242`.
- **Pattern:** holds `self.draining` (`parking_lot::Mutex`) across
  invocation of the caller-supplied `is_active_txn: impl Fn(TxnId) ->
  bool` closure inside the inner `map.retain` loop.
- **Skill guidance (Class 1):** "Don't hold a lock across a call you
  don't own. Never call user callbacks, foreign functions, or
  allocator hooks while holding a lock — they may re-enter."
- **Production call-chain verified:** `full_rebuild`
  (`core_types.rs:1322`) is called only from tests
  (`core_types.rs:4532`) and from `shared_lock_table::full_rebuild`
  (`shared_lock_table.rs:773`), which passes a closure the audit
  confirmed does **not** re-enter the `InProcessPageLockTable`.
- **Concrete-interleaving test** (per validation checklist): no real
  interleaving of current threads can reach a reentrant state. The
  closure today captures only a `TxnId → bool` map; it does not
  touch the lock table.
- **Verdict:** **architecturally safe today, FRAGILE IF REFACTORED.**
  Not a live deadlock. Filed as **bd-gq0bi** (audit/low-priority
  documentation bead, not a bug bead).
- **Suggested fix if ever promoted to a lock-reentrant caller:**
  collect `(page, txn_id)` pairs under the draining lock into a Vec,
  drop the lock, call `is_active_txn` on each outside, then
  re-acquire to apply removals. Trades one extra lock/unlock pair
  for callback safety.

## Classes checked with no findings

| Class | Result |
|---|---|
| Class 1 — classic mutex deadlock | CLEAN (consistent `tracker → arena → heads`, scoped pager locks) |
| Class 2 — async `.await` | N/A (zero `async fn`) |
| Class 3 — livelock / retry storm | No unbounded retry loops in mvcc/pager audit scope; existing CAS spins are nanosecond-scale and correctly use `spin_loop()` intrinsic |
| Class 4 — DB concurrency | Target layer itself — audited via prior sessions, loom tests in tree |
| Class 5 — LD_PRELOAD / init | N/A (no cdylib, no loader-reachable `#[no_mangle]`) |
| Class 6 — data race / TOCTOU | No new finding; existing `AtomicBool` fast-path guards paired with Mutex re-check are the correct optimistic-flag pattern, not TOCTOU |
| Class 7 — multi-process / swarm | Out of scope for in-process crates |
| Class 8 — poisoning | `std::sync::Mutex` uses surface poisoning as `FrankenError::internal`; parking_lot in mvcc doesn't poison |
| Class 9 — memory ordering | Branchless visible (ec87700b), strong CAS (4904047e), gate patterns all validated |

## Beads filed

- **bd-gq0bi** — `audit(mvcc): drain_orphaned holds draining Mutex while invoking user is_active_txn closure — callback-under-lock (currently test-only caller)`. LOW priority (P3), audit/task type. Full comment recorded on the bead.

No live-deadlock beads filed: the audit did not surface a reachable
cycle under current production call-chains.

## Validation-checklist compliance

Every finding above that was *not* escalated to a bead was discarded
because it failed at least one of the following tests:

1. **Concrete interleaving.** "Pattern matches" without a real thread
   schedule that reaches the bad state are not bugs.
2. **`&mut self` is synchronization.** Any `AtomicBool::load(Relaxed)`
   paired with a writer requiring `&mut self` is safe (borrow checker
   is the barrier).
3. **Spin-duration vs. yield cost.** Sub-microsecond critical
   sections correctly use `std::hint::spin_loop()`; recommending
   `yield_now()` there would be an anti-optimization (100–1000× slower).
4. **`Relaxed` with external synchronization.** Atomic loads whose
   synchronization comes from an enclosing Mutex, a happens-before
   from thread spawn, or `&mut self` ownership are correctly
   `Relaxed`.
5. **Double-checked gate condvar.** Fast-check → lock → re-check →
   `cv.wait` is the standard correct protocol; the post-wake check
   handles spurious wakeups; `if` (not `while`) is fine here.

## References

- `/deadlock-finder-and-fixer` SKILL.md — classification, validation
  checklist, false-positive rules.
- `docs/perf-mvcc-session-20260424.md` — this session's prior mvcc
  hot-path wins (10 commits).
- `docs/perf-mvcc-single-writer-overhead-20260424.md` — single-writer
  overhead classification and lever ladder.
