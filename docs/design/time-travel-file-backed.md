# File-backed Time Travel Decision (`bd-yaomh`)

## Status

Accepted decision: choose **Option B, a minimal `.fsqlite-history`
commit-snapshot sidecar for file-backed `FOR SYSTEM_TIME AS OF`**, then defer
the broader #30 durable-history substrate until the #82 path is production
hardened.

This note is the decision record for `bd-yaomh`. The closure gate is:

- three external reviewers or AI second-opinion runs sign off on this document:
  Boole for operator compatibility, Lagrange for decision quality, and Turing
  for storage/crash correctness,
- the follow-up P3 backlog bead exists for reviving #30 after #82 is in
  production (`bd-yaomh.8`),
- the Beads close reason records the final Option B decision in one line.

## Problem

The in-memory snapshot-ring path already verifies `FOR SYSTEM_TIME AS OF` for
`:memory:` and compatibility-runtime snapshots. File-backed historical reads
need a durable mapping from a requested historical coordinate to the database
root that represents that coordinate after close/reopen:

- `AS OF COMMITSEQ N` needs `commit_seq -> root_page`.
- `AS OF TIMESTAMP T` needs `wall_ts -> commit_seq -> root_page`.
- Historical readers need retained pages, not just retained metadata.
- Checkpoint, WAL truncation, VACUUM, replication, and rollback/downgrade must
  not silently destroy or misinterpret history.

The decision is whether to revive the full #30 durable-history substrate now or
ship a narrower log scoped to #82 first.

## Decision Drivers

The decision is weighted toward the smallest substrate that can make
file-backed time travel correct after process restart without changing the
main database format up front.

| Driver | Why it matters |
| --- | --- |
| #82 correctness | `FOR SYSTEM_TIME AS OF` must survive close/reopen and must fail explicitly when history is unavailable. |
| Concurrency | Historical reads must not reintroduce global writer serialization. Pins and retention coordinate at commit/page history level. |
| Crash safety | A torn or missing history record must be detectable and must not corrupt live recovery. |
| WAL independence | WAL checkpoint and truncation cannot be the only source of historical roots. |
| File compatibility | The first production path should avoid main-header changes unless rollback safety explicitly requires them. |
| Follow-on value | The path should keep replication, hot backup, online VACUUM, CDC, and point-in-time clone viable. |

## Options

### Option A: revive #30 in full

Revive the full durable-history substrate: durable commit metadata, a ref/pin
protocol, generic history walk, retention, checkpoint coordination, and the
wider API surface needed by replication, point-in-time clone, async snapshot
transfer, hot backup, and CDC.

Planning estimate: 12 to 20 engineer-weeks before the first production-quality
file-backed `AS OF` path, with substantial cross-crate risk.

### Option B: minimal sidecar scoped to #82

Add a sidecar commit-snapshot log at `<db>.fsqlite-history`. Each durable
commit appends the metadata needed to reopen a historical snapshot:

- `commit_seq`
- `root_page` provisional shorthand for the historical database root from which
  schema, table, and index roots are reachable
- `wall_ts_unix_nanos`
- `schema_epoch`
- checkpoint/history flags
- `database_history_id` and format generation for stale-sidecar rejection
- hash-chain fields for torn-write detection

`bd-hsi34` owns the exact record format. `bd-cd7jt` owns opening a read-only
snapshot at a historical root and registering the cross-process page pin.
Retention, VACUUM, replication, CLI, user docs, and rollback safety remain as
follow-up beads under `bd-yaomh`.

Planning estimate: 3 to 6 engineer-weeks for the first file-backed `AS OF
COMMITSEQ` path, plus the already-filed follow-up beads for full operability.

## Criteria

| Criterion | Option A: full #30 | Option B: minimal sidecar |
| --- | --- | --- |
| LOE | Months. Touches commit metadata, retention, replication, checkpoint policy, and wider history APIs before #82 can close. | Weeks. Starts with append-only metadata plus snapshot open/pin mechanics. |
| #82 payoff | Complete, but delayed by broader substrate work. | Direct. Produces the durable `commit_seq -> root_page` path #82 needs. |
| Replication / hot backup / CDC | Strong foundation if completed now. Also risks blocking #82 on features that do not need to ship first. | Requires later audits and APIs, already represented by `bd-yaomh.5` and related docs. Does not foreclose full support. |
| Online VACUUM | Can design history-aware rewrite as part of the substrate. | Must refuse, truncate, or rebuild history explicitly. `bd-yaomh.4` owns that policy. |
| Checkpoint / WAL truncation | Must coordinate deeply with checkpoint and retained WAL/history horizons. | Lookup index independent of WAL truncation; historical availability still depends on retained page versions. |
| Main file compatibility | Likely needs a broader format/version contract early. | Sidecar-only for the first path. Format-version handshake is deferred to `bd-yaomh.6` if/when needed. |
| Risk | High surface area and high chance of scope creep. | Lower surface area. Main risk is under-specifying retention/pinning semantics; that is mitigated by explicit dependent beads. |

## Payoff Matrix

| Feature | Option A now | Option B now |
| --- | --- | --- |
| File-backed `AS OF COMMITSEQ` | Yes, after full substrate lands. | Yes, after `bd-hsi34` and `bd-cd7jt`; first-class target. |
| File-backed `AS OF TIMESTAMP` | Yes, through generic metadata indexes. | Yes, after timestamp bisect work (`bd-ylm31`) on top of `wall_ts_unix_nanos`. |
| Replication | Strong native substrate. | Requires compatibility audit and backup API work (`bd-yaomh.5`). |
| Hot backup | Strong native substrate. | Needs sidecar-aware backup controls (`bd-yaomh.5`). |
| Online VACUUM | Can be integrated early. | Needs explicit policy and tests (`bd-yaomh.4`). |
| CDC | Better long-term base. | Possible later by upgrading the sidecar or reviving #30. |
| Point-in-time clone | Better long-term base. | Possible later once historical roots and retained pages are reliable. |
| Async snapshot transfer | Better long-term base. | Deferred until the minimal path proves the retention/pin model. |

## Proposed Option B Contract

The sidecar is an append-only commit-root lookup index, not a replacement for
WAL, MVCC, retained page versions, or page history. A sidecar record is valid
only if every page version reachable from that historical snapshot is durably
retained and tied to the recovered commit. If the lookup record exists but the
page-version set is missing, pruned, remapped, or not part of the recovered
history, `AS OF` must return "history not retained" instead of trying to
reconstruct or guess.

1. The main database/WAL path remains the live recovery source of truth.
2. The sidecar records the durable historical lookup surface for time travel.
3. A committed history record must be immutable once published.
4. Startup validates the hash chain and truncates only torn tail records.
5. Missing or pruned history returns an explicit "history not retained" error.
6. Historical open first acquires a retention-horizon pin keyed by
   `(database_history_id, commit_seq)` before traversal. That pin extends an
   existing retained page-version set; it does not create retention
   retroactively.
7. Checkpoint, freelist reuse, retention reapers, VACUUM, and historical
   readers must respect both persistent retention horizons and active reader
   pins.
8. Startup validates sidecar records against the recovered commit horizon and
   WAL/database generation. Records beyond the last recovered durable commit,
   or records from a mismatched sidecar/database identity, are
   torn-tail-equivalent and must be ignored or truncated before use.
9. `root_page` is provisional shorthand, not a frozen v1 promise.
   `bd-hsi34`/`bd-cd7jt` must prove whether one historical root is sufficient
   for schema, catalog, index, database-size, freelist, and VACUUM cases. If not,
   v1 must store the required root set or catalog-root coordinate before
   production use.
10. Timestamp lookup is secondary. `commit_seq` is the stable coordinate.

The first #82 implementation may avoid a broad main-file format change, but
sidecar-only does not mean downgrade-safe. Any production build that creates
`.fsqlite-history` must either pass `bd-yaomh.6` or fail closed for older
writers through an equivalent writer-exclusion/downgrade-refusal handshake
before advertising persistent historical guarantees.

## Crash and Checkpoint Semantics

Checkpoint and WAL truncation must not erase the ability to locate retained
history. The sidecar is deliberately independent as a lookup index, but not as
the retained history itself:

- WAL remains responsible for live crash recovery.
- `.fsqlite-history` remains responsible for historical root lookup.
- retained page versions remain responsible for historical page contents.
- Retention decides whether older history is still available.
- Checkpoint may mark a sidecar record as a checkpoint anchor, but it must not
  be the only record of the historical root.

Commit durability has one important policy choice for the implementation bead:
when history is enabled and the caller expects file-backed time travel, commit
success should mean the live commit, the corresponding history record, and the
retained page-version set are durable under the selected durability mode. If
the process crashes before a history record is made durable, recovery may keep
the live commit but historical `AS OF` for that commit must fail explicitly
rather than guessing. If a history record is durable but the live commit or the
referenced page versions are not recovered, the sidecar record is invalid and
must be ignored or truncated before lookup.

## Prior Art

- PostgreSQL WAL timelines separate WAL generated after point-in-time recovery
  from WAL in the original history, and small timeline history files describe
  branch points. The lesson for FrankenSQLite is that history identity must be
  explicit; if branching or point-in-time clone arrives later, `commit_seq`
  alone may need a generation/timeline discriminator.
  Source: [PostgreSQL continuous archiving and PITR](https://www.postgresql.org/docs/17/continuous-archiving.html).
- Datomic exposes an immutable database log organized in historic transaction
  order and supports range access by transaction/time coordinates. The lesson
  is that the history API should expose stable transaction coordinates and make
  time lookup a mapping onto those coordinates rather than making wall clock
  time the source of truth.
  Source: [Datomic Log API](https://docs.datomic.com/reference/log.html).
- RocksDB uses MANIFEST as a transactional log of state changes with a CURRENT
  pointer to the latest manifest. The lesson is that a small metadata log can
  reconstruct consistent storage state after restart without embedding all
  metadata in data files, but pointer/update/fsync ordering must be specified.
  Source: [RocksDB MANIFEST](https://github.com/facebook/rocksdb/wiki/MANIFEST).
- FoundationDB versionstamps show the value of a database-assigned commit
  ordering primitive. The lesson is that `commit_seq` must be assigned by the
  storage engine at commit, while wall-clock timestamps are lookup metadata with
  skew and ordering caveats.
  Source: [FoundationDB Versionstamp API](https://apple.github.io/foundationdb/javadoc/com/apple/foundationdb/tuple/Versionstamp.html).

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Sidecar says a root exists but pages were reused. | `bd-cd7jt` must add a retention-horizon pin before traversal, and the lookup must fail if the retained page-version set no longer exists. |
| VACUUM rewrites pages referenced by history. | `bd-yaomh.4` must make VACUUM preserve, rebuild, or refuse history explicitly. |
| Replication copies the main DB but omits the sidecar. | `bd-yaomh.5` must document and test replication/backup flows. |
| Older binaries open a newer history-enabled database. | `bd-yaomh.6` owns the format-version handshake and downgrade refusal. |
| A stale sidecar is copied beside a different or restored database. | `bd-hsi34` must include database/history identity and format generation fields; mismatches are hard errors before interpretation. |
| The sidecar grows without bound. | `bd-vhytr` and `bd-yaomh.7` own retention and disk-budget enforcement. |
| Timestamp lookup is ambiguous across skewed hosts. | `commit_seq` is canonical; timestamp lookup maps to the greatest retained commit at or before the timestamp and documents clock caveats. |

## Upgrade Path to Option A

The P3 follow-up bead is `bd-yaomh.8`, **"Revive #30 once #82 is in
production"**. That work should start only after:

- `bd-hsi34`, `bd-cd7jt`, and the file-backed conformance path are green,
- retention/pinning/VACUUM/replication behavior is proven under tests,
- operators can inspect and prune history through the CLI/docs path,
- production or soak evidence shows the minimal sidecar is either insufficient
  or stable enough to generalize.

At that point, the full #30 substrate can absorb the sidecar as its first
generation of durable commit metadata rather than replacing it.

## Reviewer Questions

Reviewers should explicitly answer:

1. Is a sidecar-only first path sufficient for #82 without creating hidden main
   file compatibility debt?
2. Is provisional `root_page` plus `schema_epoch` the right minimal reopening coordinate, or
   does the first format need a root set?
3. Is the commit durability policy acceptable when a live commit survives but
   its history record is missing?
4. Are the existing dependent beads enough to prevent silent breakage in VACUUM,
   replication, retention, and downgrade flows?

## Close Reason Template

After review signoff, close `bd-yaomh` with:

`Option B chosen: ship a minimal .fsqlite-history sidecar for #82 now, because
it gives durable file-backed AS OF in weeks while preserving a clean upgrade
path to full #30 after production hardening.`
