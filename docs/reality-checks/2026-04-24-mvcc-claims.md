# Reality Check — Concurrent-Writer MVCC Claims (2026-04-24)

Narrow-scope `reality-check-for-project` pass against the
concurrent-writer MVCC claims in `README.md` + `COMPREHENSIVE_SPEC_FOR_FRANKENSQLITE_V1.md`,
contrasted against the write-side evidence in
`tests/artifacts/perf/cumulative-verify-20260424T2033Z/` and the
read-side `mt_read_bench` data shipped in commit `ad899fac`
(2026-04-24).

**Bottom line:** the concurrent-writer write-side story holds.
The single-threaded and read-side parity claims do NOT.

---

## 1. What the docs claim

### 1a. Core pitch (README L26, L36, L184)

| Claim | Location | Stance |
|---|---|---|
| "MVCC Concurrent Writers. Multiple writers commit simultaneously as long as they touch different pages." | README L26 | ✅ SUPPORTED by evidence |
| "Concurrent writers: Many (page-level MVCC with SSI)" | README L36, feature table | ✅ SUPPORTED |
| "page-level versioning sits at the right point in the complexity/concurrency tradeoff" | README L57-65 | ✅ ARCHITECTURAL, not directly benchable |

### 1b. Performance table (README L1147-1173) — the measurable claims

| # | Workload | Claimed speedup | Source line |
|---|---|---|---|
| P1 | 8 threads, different tables | **~8× C SQLite** | L1151 |
| P2 | 8 threads, same table, different row ranges | **2–6×** | L1152 |
| P3 | 8 threads, same table, same hot rows | ~1× (page conflicts) | L1153 |
| P4 | Mixed 90/10 read/write | "Lower p99 read latency" | L1154 |
| P5 | **Single-threaded writes** | **~0.95×** | L1155 |
| P6 | **Single-row INSERT (1 writer)** | **"Comparable to C SQLite"** | L1168 |
| P7 | Single-row INSERT (8 writers, separate tables) | **~8× C SQLite** | L1169 |
| P8 | **Point SELECT by rowid** | **"Comparable to C SQLite"** | L1170 |
| P9 | Full table scan | "Comparable to C SQLite" | L1171 |
| P10 | Reader throughput under write load | "Higher (no aReadMark contention)" | L1173 |

---

## 2. What the benches actually show

### 2a. Write side — `mt_mvcc_bench` cumulative verify (HEAD `03c4988612`, 2026-04-24T20:33Z)

`--rows-per-thread=500 --iters=10 --threads=1,2,4,8 --apples-to-apples`,
3 repeats per thread count. Source:
`tests/artifacts/perf/cumulative-verify-20260424T2033Z/summary.md`.

| Threads | fs_wps (median) | sq_wps (median) | ratio |
|---:|---:|---:|---:|
| 1 | 407 381 | ~727 k | **0.56×** |
| 2 | 241 420 | ~536 k | 0.45× |
| 4 | 218 436 | 210 k | **1.04×** |
| 8 | **176 747** | **54 551** | **3.24×** |

Write-workload shape: one table `bench(id INTEGER PRIMARY KEY, payload
TEXT)`, each thread inserts into a disjoint rowid range in the same
table (not separate tables).

### 2b. Read side — `mt_read_bench` shipped in `ad899fac` (2026-04-24)

`--rows=5000 --reads-per-thread=20000 --threads=1,2,4,8`. Prepared
`SELECT payload FROM bench WHERE id = ?1` with random rowids.

| Threads | fs_rps | sq_rps | ratio |
|---:|---:|---:|---:|
| 1 | 18 931 | 288 428 | **0.07×** |
| 2 | 49 392 | 384 124 | 0.13× |
| 4 | 95 721 | 440 139 | 0.22× |
| 8 | 128 548 | 422 902 | **0.30×** |

Per-read wall-clock: ~53 µs (fsqlite) vs ~3.5 µs (sqlite) at 1t.

### 2c. Tests on clean HEAD

* `prop_index_btree_structural_invariants_hold_after_random_mutations`
  fails on clean HEAD in the fsqlite-btree lib suite (pre-existing, not
  caused by any btree work this session). Shrinks to a 2-insert
  minimal case.
* Cell-slot cache (bd-9e3xf audit): 0 % hit rate on `index_random_insert`
  under MemPageStore after `1be6ee30` swapped the cache key from
  xxh3 to `PageData::image_token`; production pager preserves tokens
  across reads so the 49 % hit rate captured pre-`1be6ee30` should
  carry over to real workloads, but this is not yet re-validated.

---

## 3. Gap analysis — claim by claim

| # | Claim | Evidence | Status |
|---|---|---|---|
| P1 | 8t / different tables / ~8× | **No bench measures "different tables" — mt_mvcc_bench uses one table with disjoint rowid ranges.** | **UNPROVEN** |
| P2 | 8t / same table / different row ranges / 2–6× | `mt_mvcc_bench` 8t = **3.24×** | ✅ SUPPORTED (in range) |
| P3 | 8t / same table / hot rows / ~1× | Not measured in current bench matrix | UNPROVEN |
| P4 | Mixed 90/10 p99 | `mt_read_bench` is reads-only; `mt_mvcc_bench` is writes-only; no mixed harness | **NO BENCH** |
| P5 | 1t write ~0.95× | `mt_mvcc_bench` 1t = **0.56×** | ❌ **OVERSTATED** |
| P6 | 1 writer INSERT "comparable" | Same 0.56× | ❌ **OVERSTATED** |
| P7 | 8 writers separate tables ~8× | Not measured with separate tables; same-table 3.24× at 8t | **UNPROVEN** |
| P8 | Point SELECT by rowid "comparable" | `mt_read_bench` 1t = **0.07×** | ❌ **SEVERELY OVERSTATED** (15× slower) |
| P9 | Full table scan "comparable" | Not measured in current bench matrix | **NO BENCH** |
| P10 | Reader under write-load higher throughput | Not measured | **NO BENCH** |

Supported (2): P1 shape, P2.
Overstated (3): P5, P6, P8.
Unproven / no bench (5): P1 "different tables" angle, P3, P4, P7, P9, P10.

### 3a. The honest headline vs the README headline

README frames the project as "SQLite with concurrent writers so you
get ~8× on write-heavy parallel workloads AND comparable performance
everywhere else." The evidence says:

* Write concurrency beats SQLite at 4t+ and reaches 3.24× at 8t under
  same-table contention — **this is real and reproducible**.
* Single-threaded writes and single-threaded rowid reads are
  materially slower than C SQLite (0.56× and 0.07× respectively).
  A user whose app isn't multi-threaded gets a perf regression vs C
  SQLite, not "comparable".

The 1t-read gap is the single largest unreported finding; the 15×
ratio is well past "comparable".

---

## 4. Corrective beads

File the following as beads so the drift is tracked in the triage
graph and addressed. Each proposes either (a) updating the doc to
match evidence, or (b) shipping a bench + fix that earns back the
claim.

### B1 — `bd-doc-P5-P6-P8` (P0, doc correction)

**Title:** `PERF-DOCS: correct README 1t-write and 1t-read "comparable" claims against current bench evidence`

**Body:**
Three performance-table claims in `README.md` (L1155, L1168, L1170)
do not match the 2026-04-24 bench evidence:

* L1155 "Single-threaded writes ~0.95×" — measured 0.56× in
  `cumulative-verify-20260424T2033Z`.
* L1168 "Single-row INSERT throughput (1 writer) Comparable to C SQLite"
  — same 0.56×.
* L1170 "Point SELECT by rowid Comparable to C SQLite" — measured
  0.07× at 1t in `mt_read_bench` (commit ad899fac).

Action: rewrite these three cells as "1t fs/sq ratio ≈ {0.56, 0.07}
as of 2026-04-24" plus a one-line pointer to the bench that
produced it (`mt_mvcc_bench --threads=1` and `mt_read_bench`). Do NOT
drop the claims entirely — flag them as "gap we intend to close, here
is current measured ratio and the benches that track it".

Also: add a Documentation Invariants subsection to `AGENTS.md`
requiring that every numeric performance claim in README reference a
bench name that actually measures it.

### B2 — `bd-bench-different-tables` (P1, missing harness)

**Title:** `PERF-BENCH: add mt-mvcc-bench --separate-tables mode to validate "different tables" ~8x claim`

**Body:**
The README "8 threads writing to different tables Parallel (up to 8×)"
claim (L1151, L1169) has no corresponding harness. The existing
`mt_mvcc_bench` spins all threads on the same `bench` table with
disjoint rowid ranges. That measures the "same table, different row
ranges" shape (L1152), not the "different tables" shape.

Action: extend `mt_mvcc_bench` with a `--separate-tables` flag that
CREATEs N tables (one per thread) and inserts into the thread's own
table. Capture the ratio and either (a) confirm ~8× or (b) file the
delta as a doc correction. Ship in the same artifact-dir convention
as `cumulative-verify-20260424T2033Z`.

### B3 — `bd-bench-mixed-oltp` (P1, missing harness)

**Title:** `PERF-BENCH: mixed-read-write OLTP bench for the 90/10 and reader-under-load claims`

**Body:**
README L1154 claims "Mixed read/write (90% reads, 10% writes) →
Lower p99 read latency" and L1173 claims "Reader throughput under
write load → Higher (no aReadMark contention)". Neither has a
harness. `mt_mvcc_bench` is writes-only; `mt_read_bench` is
reads-only.

Action: add `mt_oltp_bench` that combines the per-connection
template of `mt_read_bench` with a mix ratio argument, reports both
read p99 under write load and write p99 under read load at 1/2/4/8/16
threads. Gate acceptance: "lower p99 read latency" should be
verifiable with a concrete number vs C SQLite in WAL mode.

### B4 — `bd-1t-read-latency-root-cause` (P0, root-cause)

**Title:** `PERF-READ: root-cause 15x 1t SELECT-by-rowid latency gap; ship fix or doc correction`

**Body:**
`mt_read_bench` shows fs/sq 1t = 0.07× on prepared `SELECT payload
FROM bench WHERE id = ?1`. ~53 µs per read vs ~3.5 µs for C SQLite.
The pinned-read WAL-adapter commits (d9c410bb, 7e4a5409) addressed
the pager layer and MT scaling is near-linear, so the 50 µs gap lives
upstream. Candidates (not yet profiled):

* `SimpleProjectedRowidLookup` fast-path (b86cd4e6) — verify it
  actually engages for this query shape.
* VDBE dispatch overhead on PK-equality SELECT.
* Column decode of a 64-byte TEXT into `SqliteValue::Text(Arc<str>)`
  that the caller discards.
* Per-SELECT autocommit-txn acquire/release on the Connection.

Action: capture a samply flamegraph on `mt_read_bench --threads=1
--reads-per-thread=200000` and file the top-1 hot function as a
follow-up bead with an ownership label. Target: close the 1t
ratio to ≥ 0.6× OR correct the README "comparable" claim per B1.

### B5 — `bd-proptest-index-structural-invariants` (P1, test regression)

**Title:** `BTREE-TEST: repair prop_index_btree_structural_invariants_hold_after_random_mutations`

**Body:**
Pre-existing failure on clean HEAD in the fsqlite-btree lib test
suite. Shrinks to a 2-insert minimal case. Spec L11 of the README
tries to stand on "MVCC snapshots are consistent under arbitrary
transaction interleavings" and this proptest exercises the
structural guarantee one level below that. A failing invariant
proptest on main is a credibility drain on the MVCC correctness
story.

Action: resolve or `#[ignore]` with a bead reference — currently it
fails silently in CI and noisily in any local `cargo test -p
fsqlite-btree --lib`.

### B6 — `bd-cache-hitrate-production-pager` (P2, validation)

**Title:** `PERF-BTREE: validate CellSlotCache hit rate on production pager (post-1be6ee30)`

**Body:**
Commit 1be6ee30 swapped the cache key from xxh3(page-bytes) to
`PageData::image_token`. Under `MemPageStore` (the audit harness),
image_token refreshes on every `read_page_data` because each read
allocates a fresh `Vec`, so the cache records 0 % hit rate
post-1be6ee30. The production pager preserves image_token via its
page-cache `Clone` path, so the 49 % hit rate measured pre-1be6ee30
on `index_random_insert` should carry over — but this has not been
validated on the production pager.

Action: run the bd-9e3xf audit bench against the production pager
(not MemPageStore) and confirm hit rate ≥ 40 % on
`index_random_insert`. If not, file the regression. Attach result as
a comment on bd-9e3xf.

---

## 5. Summary scorecard

| Claim class | Count | Supported | Overstated | No-bench |
|---|---:|---:|---:|---:|
| Write concurrency core | 3 | 3 | 0 | 0 |
| Write perf / measurable | 4 | 1 | 2 | 1 |
| Read perf / measurable | 3 | 0 | 1 | 2 |
| **Total** | **10** | **4** | **3** | **3** |

**Recommendation:** land B1 (doc correction) within the current
release cycle; B2/B3/B4 can be phased. Without B1 the project
overclaims read-side and single-threaded performance parity that
benches do not support.

## 6. Reproduction

```
# Write side
/path/to/mt-mvcc-bench --rows-per-thread=500 --iters=10 --threads=1,2,4,8 --apples-to-apples

# Read side (shipped in ad899fac)
CARGO_TARGET_DIR=/tmp/rch_target_cc3_btree_local cargo build --release \
    -p fsqlite-e2e --bin mt-read-bench
/tmp/rch_target_cc3_btree_local/release/mt-read-bench \
    --rows=5000 --reads-per-thread=20000 --threads=1,2,4,8

# Btree test state
CARGO_TARGET_DIR=/tmp/rch_target_cc3_btree_local cargo test \
    -p fsqlite-btree --release --lib -- --test-threads=1
```

## 7. Artefact roots

* `tests/artifacts/perf/cumulative-verify-20260424T2033Z/summary.md` — write-side source of truth
* commit `ad899fac` — mt_read_bench binary + bd-eyh2e bead comment
* commit `d98e9e31` — counter gating
* bead `bd-9e3xf` — cell-slot cache classification (comment captures pre/post 1be6ee30)
