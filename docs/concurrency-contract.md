# FrankenSQLite Concurrency Contract

**Bead / Issue:** [frankensqlite#70](https://github.com/Dicklesworthstone/frankensqlite/issues/70)
**Purpose:** state, unambiguously, what concurrency guarantees FrankenSQLite
claims today — and what it does *not* — so caller projects stop re-filing
the same surface-level symptom issues every time the concurrent-write
layer drops a page or returns a stale read.

If a caller asks "does fsqlite work under N-process swarm load," the answer
should be this document plus the harness status, not a chain of point-fix
issues whose common cause was never named.

---

## TL;DR

- **Single-process, multi-Connection via MVCC WAL**: *supported*. This is
  the path every in-repo test exercises. Treat it as the default.
- **Single-process, single-Connection across threads**: *not supported by
  API*. `Connection` is `!Send + !Sync` by construction. Spawn one
  Connection per OS thread against the same file-backed database and
  coordinate below the Connection API.
- **Multi-process, multi-writer WAL**: *partial / under active hardening*.
  The `swarm_multiprocess` harness exists (`crates/fsqlite-e2e/src/bin/swarm_multiprocess.rs`)
  precisely because this path has been the source of the
  [#70 roll-up](https://github.com/Dicklesworthstone/frankensqlite/issues/70)
  family of bugs. Known failure modes are enumerated below.
- **Failing closed vs silently corrupting**: *goal*. Where a scenario is
  known to be unsafe, the runtime should emit an observable error before
  it scribbles on durable state, not after. Concrete escalation paths are
  listed under [§ Caller guidance](#caller-guidance) below.

---

## Authority

This document is advisory/normative. The machine-readable sources of
truth are:

1. **`crates/fsqlite-e2e/src/bin/swarm_multiprocess.rs`** — the
   multi-process swarm harness. Reproducible: what it asserts is what
   fsqlite currently guarantees under swarm-write load. What it fails
   on (check `crates/fsqlite-e2e/artifacts/swarm-multiprocess/` for
   captured forensic bundles) is what fsqlite does *not* yet guarantee.
2. **`crates/fsqlite-e2e/tests/correctness_concurrent_writes.rs`** and
   **`crates/fsqlite-e2e/tests/mvcc_concurrent_writers.rs`** — in-process
   multi-Connection concurrency tests.
3. **`docs/canonical_parity_contract.md`** — the SQLite-compatibility
   surface contract; multi-process WAL behavior must match stock SQLite
   for anything this doc classifies as supported.
4. **[`docs/bench-methodology-concurrent-writers.md`](./bench-methodology-concurrent-writers.md)** —
   bench-vs-correctness boundary. Read before quoting any
   throughput number from `bench_concurrent_writers`.

If this document diverges from the harness, the harness wins and this
document is out of date. Fix this doc, not the harness.

---

## The concurrency contract

### Supported: single-process, multi-Connection via MVCC WAL

- *N* Connections opened against the same file-backed database within
  a single process, coordinated through the MVCC WAL layer
  (`fsqlite-mvcc`, `fsqlite-wal`), with the Connection API per thread.
- Read-your-own-writes: a caller that receives success from `COMMIT` can
  read the committed row back on the same Connection.
- Cross-Connection visibility: after a reader's next transaction
  boundary, committed rows from other Connections are visible.
- `PRAGMA integrity_check = ok` after the workload terminates.

**Obligation on callers**: open one Connection per logical worker;
do not try to share a single `Connection` across OS threads — it is
`!Send + !Sync` and will not compile.

### Supported: multi-reader, single-writer WAL

- The classic SQLite WAL contract: readers never block, one writer at
  a time, `PRAGMA busy_timeout` governs contention.
- `busy_timeout` is honored at the Connection API. See
  [frankensqlite#45](https://github.com/Dicklesworthstone/frankensqlite/issues/45)
  for the historical `F_SETLK` gap — check its status before relying on
  cross-process busy-timeout honor.

### Partial: multi-process, multi-writer WAL

This is the path the [#70](https://github.com/Dicklesworthstone/frankensqlite/issues/70)
roll-up describes. Historical symptom families (all have been individually
patched as they were found — the meta-issue asks for a principled
root-cause sweep):

| Family | Example issues | Observable symptom |
|---|---|---|
| WAL header / checkpoint rebuild | [#19](https://github.com/Dicklesworthstone/frankensqlite/issues/19), [#56](https://github.com/Dicklesworthstone/frankensqlite/issues/56) | "WAL file too small for header during rebuild" on warm start |
| Cross-process locking | [#45](https://github.com/Dicklesworthstone/frankensqlite/issues/45) | `F_SETLK` non-blocking ignores `PRAGMA busy_timeout` |
| Process-global state leak | [#56](https://github.com/Dicklesworthstone/frankensqlite/issues/56) | "freelist trunk page exceeds db_size" on third+ open |
| Prepared-plan cache invalidation | beads_rust [#252](https://github.com/Dicklesworthstone/beads_rust/issues/252), [#254](https://github.com/Dicklesworthstone/beads_rust/issues/254), [#255](https://github.com/Dicklesworthstone/beads_rust/issues/255) | `SELECT ... WHERE pk = ?` returns zero rows (or wrong row) for freshly-committed rows; CTE wrapper masks it |
| Schema-changed-mid-INSERT | [#65](https://github.com/Dicklesworthstone/frankensqlite/issues/65) | "database schema has changed" during long rebuilds |
| FK validation on reused prepared stmt | [#59](https://github.com/Dicklesworthstone/frankensqlite/issues/59) | Wrong FK verdict after prepared statement reuse in write txn |
| Import that survives on stock SQLite | [#69](https://github.com/Dicklesworthstone/frankensqlite/issues/69) | fsqlite corrupts, stock SQLite stays clean on same import |

The common shape — "works on stock SQLite, corrupts or mis-reads on
fsqlite under concurrent load" — is what #70 asks to be addressed at
root, not via another point fix.

**Status of the fix**: tracked in `feat/*` and `fix/*` branches on
this repo (as of 2026-04: `flat-combining-page-locks`,
`fix/freelist-persist-c390`, `feat/ssi-e-process-gate`,
`fix-pager-compile`, `feat/conformal-retry-budget`,
`blackcoyote-bugA-fix`, and the MVCC optimization series
`IMPL-4`/`IMPL-14`/`IMPL-15`/`IMPL-16`/`IMPL-24`). Until those land
and the swarm harness is green at N ≥ 8 for ≥ 1 hour, treat
multi-process multi-writer as **partial**.

### Not supported today

- Cross-process advisory locks honored by non-fsqlite opener. If you
  open the same file with stock SQLite at the same time as fsqlite,
  each side's invariants are about its own pager, not the other's.
- Lock-free cross-process WAL traversal during checkpoint. Readers on
  other processes may observe transient errors during a checkpoint
  if the checkpoint coordinator has not advanced the visible watermark.

---

## Invariants the harness enforces

The `swarm_multiprocess` harness
(`crates/fsqlite-e2e/src/bin/swarm_multiprocess.rs`) is the canonical
test of every supported-scenario invariant. What it asserts:

1. **No disk-image corruption**: `PRAGMA integrity_check` post-run on
   both fsqlite and stock SQLite opening the same file.
2. **No WAL corruption**: no "WAL page index integrity failure,"
   "short header read," or frame-order anomaly after checkpoint.
3. **No silent lost writes**: a COMMIT success means the value is
   readable back by the same connection and by others after their
   next txn boundary.
4. **No stale-plan reads**: `SELECT ... WHERE pk = ?` for a freshly
   committed row returns that row (not zero rows, not a prior
   committed version).
5. **No silent wrong-row returns**: predicate matches actual row.
6. **No process-global leakage**: Nth `Connection::open` against a
   fresh file succeeds with consistent semantics.
7. **Graceful contention**: `PRAGMA busy_timeout` honored across
   processes; callers see `SQLITE_BUSY`, not hang and not zero-commit.

The canonical validation surface is:

```bash
cargo test -p fsqlite-e2e --test correctness_concurrent_writes
cargo test -p fsqlite-e2e --test mvcc_concurrent_writers
cargo run -p fsqlite-e2e --bin swarm_multiprocess -- \
    --workers 8 --seconds 60
```

When the swarm harness fails, it writes a forensic bundle
under `crates/fsqlite-e2e/artifacts/swarm-multiprocess/run-<ts>-pid<pid>/`.
Those bundles are the source-of-truth reproducers for any issue filed
against this document.

---

## Caller guidance

For caller projects (`beads_rust`, `mcp_agent_mail_rust`, anything
that opens `fsqlite` as a dependency):

### Default assumption

Operate as if you are using **single-process, multi-Connection via
MVCC WAL**. That is the supported path. If you think you need something
else, re-check — most "I need multi-process" reports turn out to be
"I have multiple callers that could run in the same process."

### If you really must open from multiple processes

- Treat the swarm as a stress test, not a production load-bearing
  contract, until the #70 fix series lands.
- Cap N at whatever your `swarm_multiprocess --workers N --seconds
  3600` run is green on. Publish that number in your caller's own
  README so downstream is not guessing.
- On startup, clean up 0–32-byte WAL sidecars before opening (see
  `mcp_agent_mail_rust/crates/mcp-agent-mail-db/src/pool.rs::cleanup_empty_wal_sidecar`
  for a tested implementation of that pattern).
- On a checkpoint failure with `WAL file too small for header during
  rebuild` or `freelist trunk page exceeds db_size`, do not fail
  closed immediately — it is an fsqlite-known recoverable class. Log,
  clean up the sidecar, and retry once before escalating.
- If your `PRAGMA integrity_check` is green but in-process verdict
  state says "corrupt," suspect the verdict classifier before
  suspecting the database. See
  [mcp_agent_mail_rust#99](https://github.com/Dicklesworthstone/mcp_agent_mail_rust/issues/99)
  for a worked example where the verdict was wrong, not the data.

### When you hit a new symptom

Run the harness against your repro:

```bash
cd $(fsqlite-repo)
cargo run -p fsqlite-e2e --bin swarm_multiprocess -- \
    --workers 8 --seconds 300 --seed "$(date +%s)"
```

If the harness is green and your application still corrupts, file an
issue with:

1. The fsqlite commit you are linking against.
2. The forensic-bundle path you captured (see above).
3. Whether stock SQLite opening the same file passes
   `PRAGMA integrity_check` (this is the key diagnostic — if stock
   SQLite also fails, the issue is probably your write path, not
   fsqlite's).
4. Your `PRAGMA`s at open time (especially `journal_mode`,
   `synchronous`, `busy_timeout`, `wal_autocheckpoint`).

A report without (3) is usually a support ticket, not a bug; (3) is
what turns a symptom into a repro against the concurrency contract.

---

## Optional opt-in: refuse multi-process access

For deployments where multi-process swarm access is known to be
unsafe (e.g., the swarm harness red on your configuration, and you
would rather fail closed than degrade), a loud-refusal opt-in would
let callers detect a second-process open and refuse with a
specific `FrankenError::MultiProcessAccessRefused` variant rather than
silently accepting it.

This is **not implemented yet**. It is sketched out in the
[#70 triage](https://github.com/Dicklesworthstone/frankensqlite/issues/70)
comment thread as a candidate follow-up. Design notes for whoever
picks it up:

- Use an advisory `fcntl(F_SETLK)` exclusive lock on a sidecar file
  (`.fsqlite-single-process-lock` next to the DB), not on the DB
  file itself — fsqlite's own cross-process locking already owns the
  DB file bytes.
- Gate behind a new `OpenFlags::REFUSE_MULTI_PROCESS` bit so the
  opt-in is explicit; default remains current behavior.
- Emit a distinctive error with the PID of the lock holder so
  operators can identify the other process, rather than a generic
  "locked" message.

When this lands, this document should grow a *"Supported: fail-closed
multi-process refusal"* section above, and the swarm harness should
grow a scenario that proves the refusal actually refuses.

---

## Change log

- **2026-04-20**: Document created in response to #70. Captures the
  contract as it stands at commit `bd770f2f` (Silo-style epoch group
  commit primitive just landed; multi-process swarm path still under
  hardening via the branches enumerated above).
