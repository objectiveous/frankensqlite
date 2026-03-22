# Changelog

All notable changes to FrankenSQLite are documented in this file.

FrankenSQLite is an independent ground-up Rust reimplementation of SQLite with
page-level MVCC concurrent writers, Serializable Snapshot Isolation (SSI), and
RaptorQ-pervasive durability. The project is organized as a 26-member Cargo
workspace under `crates/`.

> **No releases or tags exist yet.** The project is pre-release (all crates at
> 0.1.2). This changelog is organized by development phase and calendar week to
> give an agent-friendly view of what landed and when.

Repository: <https://github.com/Dicklesworthstone/frankensqlite>

---

## [0.1.2] - 2026-03-21

Version bump across all 26 workspace crates for crates.io republish.

Representative commit:
[`93f1f55f`](https://github.com/Dicklesworthstone/frankensqlite/commit/93f1f55f34a377eb8615172d7985bb5140780b2e)

---

## Development Log (pre-release)

### Week 4: 2026-03-17 -- 2026-03-21

**839 total commits on `main` as of this writing.** ~200 commits landed this
week. Focus areas: performance hot-path optimization, parallel WAL, cell-level
MVCC, group commit scaling, and WAL recovery.

#### Parallel WAL and Group Commit (Track D)

The single biggest architectural push this week. Introduced infrastructure for
multiple concurrent WAL writers and a group commit protocol that pipelines
epoch flushes.

- **Lock-free per-thread WAL buffers (D1)** -- each writer thread gets a
  dedicated WAL buffer, eliminating contention on the shared WAL append path.
  [`bf1466ce`](https://github.com/Dicklesworthstone/frankensqlite/commit/bf1466ceee2d201bbba63dba7464f7f7bcdbc7de)
- **Background epoch ticker (D1.5)** -- a dedicated thread advances the global
  epoch, decoupling WAL reclamation from writer threads.
  [`0cdc48ce`](https://github.com/Dicklesworthstone/frankensqlite/commit/0cdc48ceb1387c41b2a4d673ae7d36b4c89f8d3d)
- **Segment file I/O and recovery (D1.6, D1.7)** -- the parallel WAL is now
  backed by segment files with a recovery path.
  [`fa2745f4`](https://github.com/Dicklesworthstone/frankensqlite/commit/fa2745f4f3266129d46ceedc656d66eeb6cee6e3),
  [`712dc88a`](https://github.com/Dicklesworthstone/frankensqlite/commit/712dc88a57be3031fb56bc3d7b799ab81a1a07ac)
- **D2 ShardedPageCache** -- 128-partition page cache for thread scalability.
  [`ca3caf26`](https://github.com/Dicklesworthstone/frankensqlite/commit/ca3caf26608754fe1af7b3f3dd543ef0bbf59ea5)
- **D3 CommitSequenceCombiner** -- batched commit sequence allocation via
  flat-combining to reduce atomic contention.
  [`97e98c83`](https://github.com/Dicklesworthstone/frankensqlite/commit/97e98c83585382ec1790413678fb699c5c830072)
- **Split-lock commit protocol (D1-CRITICAL)** -- separates the commit surface
  from the conflict surface so WAL growth does not block conflict detection.
  [`1e4d6379`](https://github.com/Dicklesworthstone/frankensqlite/commit/1e4d637942d31d58dc9d0898aa5829494e36267e)
- **Epoch pipelining** -- eliminates `flushing_wait` bottleneck in group
  commit by overlapping epoch flushes.
  [`a17ba22a`](https://github.com/Dicklesworthstone/frankensqlite/commit/a17ba22ae7a618c6ea0ddf035354d8561d524fb4)
- **Page 1 conflict elimination** -- page 1 (header page) is no longer a
  mandatory conflict surface; group commit improvements.
  [`b97a3b77`](https://github.com/Dicklesworthstone/frankensqlite/commit/b97a3b777797b36cc0cbb1e32f52abdbd2c8a504)
- **RwLock WAL backend** -- WAL backend migrated from Mutex to RwLock to allow
  concurrent page reads during writes.
  [`cfd60a53`](https://github.com/Dicklesworthstone/frankensqlite/commit/cfd60a538af0657ade86edbff01102bd00ffdee5)

#### Cell-Level MVCC Visibility

The MVCC system gained cell-level (rather than page-level) visibility tracking,
enabling finer-grained conflict detection.

- **Cell-level visibility system** -- structural vs. logical page tracking,
  delta WAL, cell-level routing budgets.
  [`0094bdab`](https://github.com/Dicklesworthstone/frankensqlite/commit/0094bdab036de0cebfc0d25ec50f8637f7c912c7)
- **Cell-level visibility log and structural page tracking (C4)**.
  [`25c651e5`](https://github.com/Dicklesworthstone/frankensqlite/commit/25c651e5d57b6e5a38d4c2cd51f9e98ccf2d88c0)
- **Cell-level delta commit module** in WAL.
  [`386d641d`](https://github.com/Dicklesworthstone/frankensqlite/commit/386d641d184c2b439d178d554db80d1439d87ce1)
- **Epoch-based reclamation (EBR)** for safe concurrent version store cleanup.
  [`f050a132`](https://github.com/Dicklesworthstone/frankensqlite/commit/f050a132310387e31014233c109801ee4ddaae90)

#### Performance Hot-Path Work

Major optimization pass across VDBE, pager, and MVCC:

- **Arc\<str\>/Arc\<\[u8\]\> SqliteValue migration** -- `SqliteValue::Text` and
  `Blob` changed from `String`/`Vec<u8>` to `Arc<str>`/`Arc<[u8]>` for O(1)
  clone across the entire workspace (all 6 extension crates, func, core, VDBE,
  harness, compat layer).
  [`fa399373`](https://github.com/Dicklesworthstone/frankensqlite/commit/fa3993737120862696e26bcdc0dcfa40c4693528)
- **Hekaton-style lock-free page locks** and cached read snapshots.
  [`bb6f3606`](https://github.com/Dicklesworthstone/frankensqlite/commit/bb6f36066209de6bb71985d39c9eef399a305773)
- **Batch commit-index fence, SmallVec active-commits, proactive chain
  compaction**.
  [`55ddcc6c`](https://github.com/Dicklesworthstone/frankensqlite/commit/55ddcc6c65d7769042fb5ac076fe231f8e1e84c1)
- **Cursor-level decode cache** with hit/miss instrumentation.
  [`88c650d0`](https://github.com/Dicklesworthstone/frankensqlite/commit/88c650d06702edfe44adf9c69a9562f4fccd3fa6)
- **Zero-cost observability, in-memory pager fast path**.
  [`f44dddfb`](https://github.com/Dicklesworthstone/frankensqlite/commit/f44dddfb20803d088850c656869d46b3856445fb)
- **Per-transaction page read cache** to eliminate `inner.lock` contention.
  [`cc8a47aa`](https://github.com/Dicklesworthstone/frankensqlite/commit/cc8a47aadbf9ee5261854065fcb8ca2e6251121e)
- **Batch-allocate EOF pages** to reduce mutex contention during concurrent
  inserts.
  [`878e8215`](https://github.com/Dicklesworthstone/frankensqlite/commit/878e8215f63de2758af6f9d760dadf7edc877539)

#### WAL Recovery and Compatibility

- **WAL-recovery for stale main-file headers** and read-only WAL backend
  install.
  [`6da92596`](https://github.com/Dicklesworthstone/frankensqlite/commit/6da9259684813dfe2ed0e5ff62c0edcd971b9ffa)
- **Centralized WAL backend installation** with page-size validation and owned
  adapter handoff.
  [`ea2ff736`](https://github.com/Dicklesworthstone/frankensqlite/commit/ea2ff736c5186aa3132c475dc79fb6bf9a42ea40)
- **SSI CommittedPivot detection, ghost epoch tracking, cache invalidation**.
  [`d53bd9c6`](https://github.com/Dicklesworthstone/frankensqlite/commit/d53bd9c6e8c5f4f7dfecf3564b4665fe33601a75)

#### C API and WASM

- Continued C API compatibility expansion -- temporary database lifecycle,
  finalize error propagation, VDBE result code parsing.
  [`20b587f3`](https://github.com/Dicklesworthstone/frankensqlite/commit/20b587f385a38902d16aed86ec12ce0d75bceafd)
- WASM compatibility maintained throughout Arc migration.

#### Other Notable Fixes

- B-tree: handle oversized interior cell replacement via structural rebalance.
  [`f417dcad`](https://github.com/Dicklesworthstone/frankensqlite/commit/f417dcad546aff15269ec19ce0c271d0a118057e)
- Native `Connection::execute_batch` with no-op detection.
  [`1021ead3`](https://github.com/Dicklesworthstone/frankensqlite/commit/1021ead32742266822b529c688d8009e0724de2e)
- Property-based coverage for cell visibility invariants (proptest).
  [`6f5582f6`](https://github.com/Dicklesworthstone/frankensqlite/commit/6f5582f69182d29c0dcc77ab9b144bccda3ee4a5)

---

### Week 3: 2026-03-10 -- 2026-03-16

~220 commits. Focus areas: extension parity, conformance oracle expansion,
MVCC left-right publication, differential testing, async dispatch, and
performance profiling.

#### Extension Parity Push

All major SQLite extensions got significant work:

- **FTS5** -- real column filter evaluation, `highlight()` and `snippet()`
  scalar functions.
  [`0147eb5b`](https://github.com/Dicklesworthstone/frankensqlite/commit/0147eb5b4b8c5b1876a7fde6afe2ea5ee663498d),
  [`3f6d1189`](https://github.com/Dicklesworthstone/frankensqlite/commit/3f6d1189985f6bb7751ccf00027448d878bc2c2b)
- **R-Tree** -- full virtual table adapter with harness matrix coverage.
  [`d48445e7`](https://github.com/Dicklesworthstone/frankensqlite/commit/d48445e7667f88209c31ef1529e73bed6ea32aa6)
- **JSON/JSONB** -- JSONB scalar function parity and blob input support.
  [`fbfe5675`](https://github.com/Dicklesworthstone/frankensqlite/commit/fbfe5675ece6c2d3cf2f753f66b30f54f3821ce5)
- **Session** -- explicit primary key requirement enforcement.
  [`eee5902b`](https://github.com/Dicklesworthstone/frankensqlite/commit/eee5902be8d6e9094464244295c821047cb29f6a)
- **Full WASM database engine** with R-tree virtual table adapter.
  [`f76c7de2`](https://github.com/Dicklesworthstone/frankensqlite/commit/f76c7de2a55d7a4aa9b869ba526388664dcb2fb4)

#### SQL Feature Expansion

- **VACUUM INTO** implementation.
  [`f96986a4`](https://github.com/Dicklesworthstone/frankensqlite/commit/f96986a4dc87dc696d05e4eb5943278e355eb9c0)
- **ANALYZE/REINDEX** with `sqlite_stat1` support.
  [`825cb634`](https://github.com/Dicklesworthstone/frankensqlite/commit/825cb6349980c1e3fdb2f5200883fb5ad6e8f005)
- Expression index and partial index support in codegen and query planning.
  [`4a857675`](https://github.com/Dicklesworthstone/frankensqlite/commit/4a857675a962e35443d9e5918a868cd1d0a13dd1)
- Schema generation tracking, virtual table module registry, and prepared
  statement dispatch.
  [`6526d935`](https://github.com/Dicklesworthstone/frankensqlite/commit/6526d935e23365f78fc4b5dc3121c3902b4b7cd6)
- In-aggregate ORDER BY and trigger RAISE evaluation.
  [`3eb6aaec`](https://github.com/Dicklesworthstone/frankensqlite/commit/3eb6aaec2cdb843de04548b13256ef2f4011c5d8)
- Robust ALTER TABLE schema fidelity for rename-column and add-column.
  [`3b9848a6`](https://github.com/Dicklesworthstone/frankensqlite/commit/3b9848a649420d91859d15c66b9ac45f44d74c02)

#### MVCC and Concurrency Advances

- **CommitIndex migrated to left-right publication** -- replaces RwLock shards
  for lock-free reads.
  [`1efe6740`](https://github.com/Dicklesworthstone/frankensqlite/commit/1efe6740b0ed321d74ee58351a186a898e0c7316)
- **Distributionally Robust Optimization (DRO)** layer for SSI T3 abort
  decisions.
  [`10b5e45c`](https://github.com/Dicklesworthstone/frankensqlite/commit/10b5e45cf45a1691d420784af7249617070f54b4)
- **DroVolatilityTracker** for sliding-window DRO radius estimation.
  [`d598a108`](https://github.com/Dicklesworthstone/frankensqlite/commit/d598a108e4e977197e2769388a3be3a941064303)
- **HTM abort metrics, RTRIM collation dedup**.
  [`8ce58bbd`](https://github.com/Dicklesworthstone/frankensqlite/commit/8ce58bbdf75c7f6d0bf0111186151782b609c69a)
- BOCPD and conformal martingale input sanitization.
  [`bb72b151`](https://github.com/Dicklesworthstone/frankensqlite/commit/bb72b151d7714cea50a2b5124e8a5d53af8fc591)
- Asupersync runtime migration for commit repair and benchmarks.
  [`4730202e`](https://github.com/Dicklesworthstone/frankensqlite/commit/4730202e43bb25bbd619688b4a02dc853c59abfd)

#### Pager and WAL Refinements

- WAL adapter overhaul with FEC pipeline expansion.
  [`c0c5ae33`](https://github.com/Dicklesworthstone/frankensqlite/commit/c0c5ae33222fe701b372ab7edea35f9552d9d54d)
- Split prepared-frame append into pre-lock finalize and durable write phases.
  [`ea3e9e00`](https://github.com/Dicklesworthstone/frankensqlite/commit/ea3e9e0005c4ed8325f262b4632f5d8057518a73)
- Affine checksum transforms and frame scratch reuse.
  [`5c76d1d8`](https://github.com/Dicklesworthstone/frankensqlite/commit/5c76d1d84d59b6c6d7b9acc376a5a8b65f01a807)
- WAL page index ABA hazard prevention via generation identity tracking.
  [`2df16c8e`](https://github.com/Dicklesworthstone/frankensqlite/commit/2df16c8e3eebe276fb4da05393681f0dcfa80a0b)
- Cache-line-striped atomics for publication counters.
  [`e7612b1b`](https://github.com/Dicklesworthstone/frankensqlite/commit/e7612b1b6e5b4f72d48c5fead4eff42ccf7eb58d)

#### C API

- Multi-statement batch execution and real column names in `sqlite3_exec`.
  [`d40e4cb2`](https://github.com/Dicklesworthstone/frankensqlite/commit/d40e4cb27d29a4050884bf1cb8455bb2e546afbb)
- `sqlite3_prepare_v2`-style statement parsing with tail offset support.
  [`cf10270d`](https://github.com/Dicklesworthstone/frankensqlite/commit/cf10270d125313398ce5761af1eaf8df87a4cfcd)

#### Async and Performance

- Async dispatch, channel-free SSI ledger.
  [`4074edb5`](https://github.com/Dicklesworthstone/frankensqlite/commit/4074edb5a1719f7ad855b5c31cac50b75d5a10c6)
- Reduce per-statement allocation overhead on autocommit hot paths.
  [`63fdd78c`](https://github.com/Dicklesworthstone/frankensqlite/commit/63fdd78c9f4907d53b7d3ce4dfce90ecddb49c6b)
- Sort-based GROUP BY, `Arc<Statement>` in prepared stmts, NOCASE collation
  optimization.
  [`69e8af20`](https://github.com/Dicklesworthstone/frankensqlite/commit/69e8af20d5141f3b32ad4e81570667deec8c8f41)

---

### Week 2: 2026-03-03 -- 2026-03-09

~190 commits. Focus areas: SQL conformance oracle, concurrency hardening,
window functions, time-travel queries, and SSI correctness.

#### Conformance Oracle Explosion

A massive conformance test campaign produced 400+ oracle tests. Each test
runs the same SQL against both FrankenSQLite and C SQLite, asserting identical
results.

- From ~0 to 400+ oracle conformance tests in a single week. Tests cover
  JOINs, aggregates, window functions, subqueries, CTEs, triggers, foreign
  keys, UPSERT, DISTINCT, COLLATE, type coercion, and dozens of edge cases.
- Representative batch commits:
  [`11665f60`](https://github.com/Dicklesworthstone/frankensqlite/commit/11665f60da9ba1d61ac42577d7df954032f74f01) (200 total),
  [`8cf6f075`](https://github.com/Dicklesworthstone/frankensqlite/commit/8cf6f0752fd93f977da844d2e8396516b73d5cfe) (353 total)

#### Window Functions

Full window function support landed:

- NTH_VALUE, CUME_DIST, PERCENT_RANK evaluation with RANGE peer-group
  semantics.
  [`25cfd93e`](https://github.com/Dicklesworthstone/frankensqlite/commit/25cfd93e7652291b801eeac981e964e6d5699a92)
- Per-function partition/sort for multiple window specs.
  [`12638ce9`](https://github.com/Dicklesworthstone/frankensqlite/commit/12638ce9cf5cb327e626ae52d8987761ef9cffcb)
- ORDER BY NULLS FIRST/LAST support.
  [`725bd298`](https://github.com/Dicklesworthstone/frankensqlite/commit/725bd298642d33386d984c0f047def98bb5c0a6a)
- Two-pass evaluation for all partition-dependent window functions.
  [`0b39083c`](https://github.com/Dicklesworthstone/frankensqlite/commit/0b39083c089d5f71181cb4f553df21d2db3dda96)

#### Time-Travel Queries (SQL:2011 Temporal)

- SQL:2011 temporal query parsing and MVCC snapshot resolution.
  [`29f6dbea`](https://github.com/Dicklesworthstone/frankensqlite/commit/29f6dbeabd2f52fe1d734972c678b59d1c3281f1)
- `SetSnapshot` opcode for temporal queries.
  [`5c852551`](https://github.com/Dicklesworthstone/frankensqlite/commit/5c852551a045bc2c56725aa0854c73c4c070265f)
- End-to-end time-travel queries wired through MVCC.
  [`69b57ecf`](https://github.com/Dicklesworthstone/frankensqlite/commit/69b57ecf8572f06dca6f9a58cb12d6fd4e1b5f0e)

#### MVCC and SSI Hardening

- Adaptive checkpoint scheduling with advisor PRAGMAs.
  [`191ecaeb`](https://github.com/Dicklesworthstone/frankensqlite/commit/191ecaeba903077a95e0c27f4f33ba8d21c18690)
- Prevent spurious FK cascade on UPDATE when PK unchanged.
  [`e039e693`](https://github.com/Dicklesworthstone/frankensqlite/commit/e039e693a0260a872f5ec917b0882d6d16cb40d5)
- Balance-quick page leak fix, TOCTOU race fix in serialized writer lock.
  [`9ba09190`](https://github.com/Dicklesworthstone/frankensqlite/commit/9ba091907c28e86f857ac915fba35920c65aa407)
- Replace O(n*k) brute-force segment training with O(n) slope-constraint PLA
  in B-tree learned index.
  [`22fb1dae`](https://github.com/Dicklesworthstone/frankensqlite/commit/22fb1daedc5140ba79d75233ed2ebbfb9e531322)

#### SQL Correctness Fixes

- Kahan-Babuska-Neumaier compensated summation for SUM/AVG/total precision.
  [`d5ac7704`](https://github.com/Dicklesworthstone/frankensqlite/commit/d5ac770473ee9242282da0e1a6e6ebdf8903e2b0)
- CHECK constraint enforcement on INSERT and UPDATE.
  [`3db8d9ae`](https://github.com/Dicklesworthstone/frankensqlite/commit/3db8d9aee955018d79b79da9ff591ced270ad2df)
- NOT NULL constraint enforcement at codegen level.
  [`573a1006`](https://github.com/Dicklesworthstone/frankensqlite/commit/573a1006f4815e13167df204a4bb9c55db18aa8f)
- IS TRUE/FALSE emit `IsTrue` opcode.
  [`e3ab41cd`](https://github.com/Dicklesworthstone/frankensqlite/commit/e3ab41cd0e5d35d95a267d4aa59238f49047449d)
- Type affinity coercion in comparisons.
  [`84e01813`](https://github.com/Dicklesworthstone/frankensqlite/commit/84e018131715652ad301793754e4d11c7cb08319)
- Propagate COLLATE NOCASE to IN/BETWEEN codegen.
  [`1973ed92`](https://github.com/Dicklesworthstone/frankensqlite/commit/1973ed92608a720239d4fa0b351b514c7788831a)
- CTE support for UPDATE/DELETE, collation-aware DISTINCT.
  [`603d4296`](https://github.com/Dicklesworthstone/frankensqlite/commit/603d42961020e58417347d281335b92fab433d0c)

#### io_uring

- Wired `io_uring` as default Linux pager backend.
  [`00f4a6ac`](https://github.com/Dicklesworthstone/frankensqlite/commit/00f4a6ac64cfb3cfdc597528435223ca02c2d238)

#### Other

- Record parsing hardening, pager rollback journal robustness, WAL checksum
  fix.
  [`5d6ab1f1`](https://github.com/Dicklesworthstone/frankensqlite/commit/5d6ab1f135f5a1b3e5e631e026b2ef8d9369c496)
- WAL generation tracking and conformance tests.
  [`23bf7586`](https://github.com/Dicklesworthstone/frankensqlite/commit/23bf75862a6376daec67bf60c3e2a7036af4f057)
- Write witness SSI, precompiled program reuse, CAMP morsel optimization.
  [`728dd358`](https://github.com/Dicklesworthstone/frankensqlite/commit/728dd3584f676a6c99fded6739c1d597d84bad31)

---

### Week 1: 2026-02-25 -- 2026-03-02

~170 commits. The project transitioned from specification/scaffolding to
active engine development. Core SQL execution, MVCC, B-tree, VDBE, and pager
all became functional in this week.

#### Core SQL Execution

- **BEGIN CONCURRENT** -- initial concurrent transaction support with e2e
  tests.
  [`e803849a`](https://github.com/Dicklesworthstone/frankensqlite/commit/e803849a232f8452bd0daf27305bb2a1de356895)
- **20+ VDBE opcodes** including UNIQUE index codegen, collation-aware
  comparison, and type affinity.
  [`6c4d9664`](https://github.com/Dicklesworthstone/frankensqlite/commit/6c4d966471b57389f2b79da38ba2b8cc2a0fa740)
- **AUTOINCREMENT/sqlite_sequence** support, UNIQUE constraint enforcement.
  [`bcb357a7`](https://github.com/Dicklesworthstone/frankensqlite/commit/bcb357a71d97752a8165b437923494d8eada1347)
- **VALUES clause, SELECT-without-FROM, DISTINCT aggregates**.
  [`f6ba2322`](https://github.com/Dicklesworthstone/frankensqlite/commit/f6ba2322ab1119a9f068a194aaa47fc3fe05b39f)
- **FILTER clause, correlated subquery detection**.
  [`b2911871`](https://github.com/Dicklesworthstone/frankensqlite/commit/b29118712afd66ffbf19d84ef2cd82f9bde0c4a6)
- **Generated columns** (stored and virtual).
  [`2b4503ec`](https://github.com/Dicklesworthstone/frankensqlite/commit/2b4503ece38e87b7ee26457e059505acdb0be3fb)
- **IS [NOT] DISTINCT FROM, NOT NULL postfix**.
  [`bbee1add`](https://github.com/Dicklesworthstone/frankensqlite/commit/bbee1addded154cfb1f0cde8a1e2c44f97898b3f)
- **Virtual table opcodes**, INSERT conflict resolution, and comparison
  affinity coercion.
  [`cde891c7`](https://github.com/Dicklesworthstone/frankensqlite/commit/cde891c7c8daef87e4339888eb251e3eb4cae85c)
- **SQL:2011 temporal query parsing** and time-travel field propagation.
  [`29f6dbea`](https://github.com/Dicklesworthstone/frankensqlite/commit/29f6dbeabd2f52fe1d734972c678b59d1c3281f1)
- **FK enforcement** on UPDATE/DELETE.
  [`d314d32b`](https://github.com/Dicklesworthstone/frankensqlite/commit/d314d32bcd46b4eabc7a2b80c06b21ec2a03fc94)
- **JIT scaffold** with hot-query tracking and PRAGMA controls.
  [`c0fae9a2`](https://github.com/Dicklesworthstone/frankensqlite/commit/c0fae9a20e47385024873a07bb8f8c469117c73f)
- **UPSERT/ON CONFLICT**, STRICT type enforcement, parse cache.
  [`e753b7bb`](https://github.com/Dicklesworthstone/frankensqlite/commit/e753b7bb0445c690be8c7d85ee698bac8bd0f922)

#### MVCC Foundation

- Version-chain length controls with eager GC and backpressure.
  [`ef3a472e`](https://github.com/Dicklesworthstone/frankensqlite/commit/ef3a472e683b6b4ef12483dec7271674e4a7b207)
- Stabilize IPC on nix crate and fix correctness issues across mvcc, pager,
  btree, func, vdbe, types.
  [`e6169106`](https://github.com/Dicklesworthstone/frankensqlite/commit/e61691066de0f63f0b70dc49cd293564f6b560ca)
- Move `plan_concurrent_commit` under `commit_write_mutex` to prevent MVCC
  commit index corruption.
  [`8cbdd1e0`](https://github.com/Dicklesworthstone/frankensqlite/commit/8cbdd1e0174e632848ed025ced58c7ffc4cbd1b5)

#### B-Tree

- UNIQUE index enforcement and record comparison semantics.
  [`fff4cbb5`](https://github.com/Dicklesworthstone/frankensqlite/commit/fff4cbb52fb948ef624f82fc23f372f19699d909)
- Interior-node deletion.
  [`455043bd`](https://github.com/Dicklesworthstone/frankensqlite/commit/455043bdd0d5dc3045fdd44a69d463d77f7cc6cf)

#### Pager

- Page1 header patching on commit to prevent malformed DB.
  [`1ab7cee6`](https://github.com/Dicklesworthstone/frankensqlite/commit/1ab7cee6670ba87a69a3eaf1fc8ca307f15b7a03)
- Evict page 1 from cache after Phase 2b header patching in journal commit.
  [`59e73d24`](https://github.com/Dicklesworthstone/frankensqlite/commit/59e73d24da5244929a377e62d237627e4c948cf4)
- Persist freelist to SQLite freelist pages.
  [`a7c95f42`](https://github.com/Dicklesworthstone/frankensqlite/commit/a7c95f4203051ec2d9be253ffdd3c24c24501f89)

#### Conformance and Testing

- Parity-certification mode, MVCC visibility telemetry, WAL replay tracing.
  [`84d7b1a6`](https://github.com/Dicklesworthstone/frankensqlite/commit/84d7b1a6a7851d6583d8d961ad07bf3a1d12c741)
- Oracle preflight doctor in CI.
  [`6f491bf8`](https://github.com/Dicklesworthstone/frankensqlite/commit/6f491bf814b229ea351c814c0b3b390e9bc03baf)
- Exhaustive function parity matrix differential test against C SQLite.
  [`4c9cf08e`](https://github.com/Dicklesworthstone/frankensqlite/commit/4c9cf08e9a9c078eb0a75453c9da2ab3318d2c7a)
- Crash-loop replay determinism test for WAL recovery.
  [`3675601a`](https://github.com/Dicklesworthstone/frankensqlite/commit/3675601a82d48a6979dd99c0dd54e9e49023d96c)

#### Type System and Functions

- Match SQLite's `%!.15g` float-to-text formatting.
  [`6ae07957`](https://github.com/Dicklesworthstone/frankensqlite/commit/6ae079572979023507c7fc2cfc73f696a4fd5739)
- SQL three-valued NULL logic for comparisons.
  [`44b6f1dc`](https://github.com/Dicklesworthstone/frankensqlite/commit/44b6f1dc2cdcdeb568e2be20c42963378c251327)
- Localtime/utc modifiers and month/year overflow in datetime.
  [`0a269b5b`](https://github.com/Dicklesworthstone/frankensqlite/commit/0a269b5b9a68a5740c4a9c0209782e29e7c01117)

---

### Pre-Development: 2026-02-06 -- 2026-02-07

~1,660 commits in the `--all` view (many on side branches). This phase
produced the specification documents, workspace scaffolding, and formal MVCC
proofs. No runnable engine code yet.

#### Workspace Scaffolding (Feb 6)

- **Initialize 23-crate Rust workspace** with resolver v2, `#[forbid(unsafe_code)]`
  globally, and clippy pedantic+nursery at deny level.
  [`a137671e`](https://github.com/Dicklesworthstone/frankensqlite/commit/a137671e2e7c4b25547d24e540d72f69a5c9efe1)
- **fsqlite-types**: core type system with 64 tests.
  [`bfd62701`](https://github.com/Dicklesworthstone/frankensqlite/commit/bfd62701858561f59913a2d61a966d7dcc239152)
- **fsqlite-error**: structured error handling with 13 tests.
  [`b68faa8c`](https://github.com/Dicklesworthstone/frankensqlite/commit/b68faa8cb960d215e9834c66021e3f5aa6d3827e)
- Storage layer stubs (vfs, pager, wal, mvcc, btree), SQL processing stubs
  (ast, parser, planner, vdbe, func), extension stubs (fts3, fts5, rtree,
  json, session, icu, misc), and integration layer stubs.
  [`2d5283b4`](https://github.com/Dicklesworthstone/frankensqlite/commit/2d5283b4e0ab91bc687e4b9cfae4cb84ad4a0470)
  through
  [`b559f58e`](https://github.com/Dicklesworthstone/frankensqlite/commit/b559f58e426d995f4ba101ecd80096977b9834f4)
- SQLite C source added as git submodule for behavioral reference.
  [`42f23e15`](https://github.com/Dicklesworthstone/frankensqlite/commit/42f23e157fda2ca29d2e219b8e8dbdc9bdd5377a)

#### Specification Documents (Feb 6-7)

- Comprehensive SQLite behavioral specification document.
  [`1d40327f`](https://github.com/Dicklesworthstone/frankensqlite/commit/1d40327feb3aefa88065654accac2155a40cfca5)
- Formal MVCC specification with proofs and implementation order.
  [`8841a3ec`](https://github.com/Dicklesworthstone/frankensqlite/commit/8841a3ec70cac0eec5ea626186d435ffd4287795)
- Comprehensive specification documents (8,628 + 1,206 lines).
  [`c08f1602`](https://github.com/Dicklesworthstone/frankensqlite/commit/c08f1602d03b1833a4f91c8f77347f8f196bac9d)
- RFC 6330 (RaptorQ) reference document.
  [`c293739f`](https://github.com/Dicklesworthstone/frankensqlite/commit/c293739fccb9d88a948f1d151b8fcf877424760d)
- Spec evolved through V1.3 -> V1.4 -> V1.5 -> V1.6a-h -> V1.7a-j with deep
  audit rounds covering MVCC formal model, checksums/integrity, buffer pool
  (ARC cache), SQL coverage, RaptorQ MTU/sub-blocking, and e-process math.
- Beads issue tracker initialized with 92 work items across all phases.
  [`be5dc72e`](https://github.com/Dicklesworthstone/frankensqlite/commit/be5dc72edf86b3f831eea68d729cb5aed0a43034)

---

## Workspace Crates

| Crate | Role |
|-------|------|
| `fsqlite` | Top-level public API facade |
| `fsqlite-core` | Connection, query dispatch, schema management |
| `fsqlite-types` | Core type system (`SqliteValue`, `PageNumber`, `TxnId`, etc.) |
| `fsqlite-error` | Structured error types |
| `fsqlite-vfs` | Virtual File System (POSIX, io_uring, WASM) |
| `fsqlite-pager` | Page cache, group commit, WAL integration |
| `fsqlite-wal` | Write-Ahead Log (compat + parallel) |
| `fsqlite-mvcc` | Page-level MVCC, SSI, EBR, version store |
| `fsqlite-btree` | B-tree engine with learned index |
| `fsqlite-ast` | SQL abstract syntax tree |
| `fsqlite-parser` | SQL parser |
| `fsqlite-planner` | Query planner and optimizer |
| `fsqlite-vdbe` | Virtual Database Engine (bytecode interpreter) |
| `fsqlite-func` | Built-in scalar and aggregate functions |
| `fsqlite-ext-fts3` | FTS3 extension |
| `fsqlite-ext-fts5` | FTS5 extension |
| `fsqlite-ext-rtree` | R-Tree extension |
| `fsqlite-ext-json` | JSON/JSONB extension |
| `fsqlite-ext-session` | Session extension |
| `fsqlite-ext-icu` | ICU extension |
| `fsqlite-ext-misc` | Miscellaneous extensions (`generate_series`, etc.) |
| `fsqlite-c-api` | Optional C ABI shim (only `unsafe` code in workspace) |
| `fsqlite-cli` | Command-line shell |
| `fsqlite-e2e` | End-to-end tests and benchmarks |
| `fsqlite-harness` | Conformance test harness and oracle infrastructure |
| `fsqlite-wasm` | WebAssembly database engine |
| `fsqlite-observability` | Telemetry and instrumentation |
