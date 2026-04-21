# PERF-A1 MemDatabase Row Values Conclusion

## First attempt: Arc<[SqliteValue]> container swap (rolled back)

Bead `bd-1dp9.6.7.11`: the pre-A1 symbolized flamegraph showed `SqliteValue` slice-to-Vec cloning as visible (`<fsqlite_types::value::SqliteValue as <[_]>::to_vec_in::ConvertVec>::to_vec`, 52,223,806 samples, 1.06%), with surrounding `MemTable`/`MemDatabase` clone and insert frames, so the hotspot was real; however the least-invasive `Arc<[SqliteValue]>` row-value patch compiled (`rch exec -- cargo check -p fsqlite-vdbe -p fsqlite-core --all-targets`) but regressed the target benchmark on the same machine: reverted pre-change `hyperfine --warmup 1 --runs 7 --show-output '/tmp/cargo-target/release-perf/perf-update-delete 10000 10 both'` reported `264.6 ms +/- 3.9 ms`, while the patched build reported `271.5 ms +/- 4.5 ms`, so the API/container churn was rolled back and should not ship as A1.

## Re-framing as a time-travel snapshot design problem

The Arc swap regressed because it replaced one small cost (a short `Vec<SqliteValue>` memcpy during insert) with a larger distributed cost (atomic refcount increments/decrements every time a row is cloned, handed to another layer, or dropped — including hot paths that previously did nothing because the receiving layer already owned a Vec). Atomic inc/dec on a shared cache line is 10-30ns on x86; a 40-byte `Vec` memcpy is <5ns at L1. For rows with ≤8 values the clone is actually cheaper than `Arc::clone`, and the hot path fires on every insert while refcount pairs fire on every copy-read.

Two alternative shapes were considered. Both are feasible but not worth landing:

### (A) Per-txn bumpalo arena for row values

MemTable stores rows as `ArenaRow<'txn>` pointing into a bumpalo arena owned by the committing transaction. At commit, the arena is promoted to an `Arc<Bump>` and hung off the MemDatabase snapshot. The arena is freed when no snapshot references it (epoch GC).

Savings: eliminates the per-value SqliteValue copy into MemTable (the 0.38% self-time band).
Costs:
- All consumers of `&MemRow` now need a lifetime or the MemTable's current `Arc<Bump>`.
- SqliteValue variants that own heap data (`Text`, `Blob`) would need to be rewired to intern into the arena, otherwise the row is half-arena/half-heap and the value copy is merely moved rather than eliminated.
- Cross-txn row reads (the common case for a SELECT) would need to walk an arena chain or materialise into a foreground buffer — adding a cost equal to what's saved.

### (B) Snapshot-log MemDatabase with copy-on-write at table granularity

Each MemTable is `Arc<TableSnapshot>`. A commit that modifies a table creates a new snapshot via persistent-data-structure apply (HAMT / tidy-tree / chunked-vector). Readers take an Arc clone at begin_transaction; writers publish a new snapshot at commit.

Savings: eliminates the row-value copy for unchanged rows (readers share the prior Arc). Changed rows are still copied, but only in the modified table.
Costs:
- Persistent-data-structure apply introduces Arc atomics on every table-level snapshot promotion, which is the exact cost that made (A) regress.
- Memory churn: snapshot log grows unboundedly until the oldest reader drops; epoch GC adds bookkeeping on every `begin_transaction` / `end_transaction`.
- Large ripple: MemTable, MemDatabase, InsertRow / DeleteRow / UpdateRow, and every VDBE opcode that materialises a row from MemDatabase would need to be re-expressed as "take snapshot, then query".

### (C) Parse directly into MemDatabase row slots

`parse_record_into` already accepts `&mut Vec<SqliteValue>` and reuses scratch capacity (comment at `crates/fsqlite-core/src/connection.rs:6102`). On the `reload_memdb_from_txn` path, instead of calling `parse_record` → returning a fresh `Vec<SqliteValue>` → moving that Vec into MemTable row storage, parse directly into the destination row slot using `parse_record_into` against the MemTable's already-allocated row Vec. This is the narrowest copy-elimination available: it saves the one-Vec hand-off without touching the ownership model.

Savings: estimated at 0.20-0.30% self-time (half of the observed clone band) based on where the clone frame sits in the flame.
Costs: requires MemTable to expose a "reserve a row slot and return `&mut Vec<SqliteValue>`" API, which is viable but still a cross-crate change (MemTable lives in fsqlite-vdbe, reload_memdb lives in fsqlite-core).

## Decision

Don't ship any of (A), (B), or (C) as PERF-A1:

- (A) and (B) fail the Score ≥ 2.0 gate (Impact × Confidence / Effort). Impact is ~1% best case, Confidence is low after the Arc regression, Effort is multi-day cross-crate refactoring with aliasing/lifetime subtleties that put snapshot isolation and rollback safety at risk.
- (C) scores better (Impact ~0.3%, Confidence medium, Effort low-medium) but still below the 0.5% ship threshold that the MEMCPY band lives in. Revisit if the surrounding clone stack grows or if a future change makes the `parse_record` → Vec hand-off more expensive (e.g. a wider SqliteValue that pushes the memcpy into the allocator slow path).

Re-profile after any change that touches MemTable or reload_memdb. If the band drifts above 1% again, (C) is the first thing to try; (A) and (B) remain off-limits until someone lands table-level snapshot versioning for an independent reason (e.g. cell-level MVCC work on bd-l9k8e).

## Artifacts

- Pre-A1 symbolized flamegraph: `flamegraph-pre-a1-symbols.svg` (local, not committed — 747 KB)
- Bench deltas: 264.6 ± 3.9 ms (pre) vs 271.5 ± 4.5 ms (Arc<[SqliteValue]> patch, regressed → reverted)
- Relevant code paths: `crates/fsqlite-core/src/connection.rs::reload_memdb_from_txn_with_mode` (line ~43346), `crates/fsqlite-vdbe/src/engine.rs::MemTable` (line ~595), `parse_record_into` in `fsqlite-types`.
