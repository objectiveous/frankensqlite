# Agent-Swarm SQL Coordination API Contract

Date: 2026-05-24
Bead: `bd-agent-swarm-coordination-transparency-8jr6u.2`
Status: SQL-visible contract for queue, lease, range, and diagnostic surfaces

## Purpose

This document fixes the SQL shape for FrankenSQLite coordination primitives
before parser, planner, VDBE, storage, or harness implementation starts. The
surface is for user workloads that run many autonomous workers against one
database and need transactional coordination through SQL.

The API must preserve the central FrankenSQLite invariant: concurrent-writer
mode stays on by default, plain `BEGIN` still promotes to `BEGIN CONCURRENT`
unless the caller explicitly opts out, and queue, lease, and worker-range
ownership must not introduce a global file or connection writer lock.

## Surface Decision

The first public surface is a set of writable virtual catalog tables plus
normal SQL DML and PRAGMA diagnostics:

- `fsqlite_queue`
- `fsqlite_lease`
- `fsqlite_worker_ranges`
- `fsqlite_coordination_events`
- `PRAGMA fsqlite_coordination_reason_codes`
- `PRAGMA fsqlite_coordination_status`
- `PRAGMA fsqlite_coordination_reset`

The queue, lease, and range tables are built-in virtual tables backed by
FrankenSQLite internal catalog storage. Callers mutate them with ordinary
`INSERT`, `UPDATE ... RETURNING`, and `DELETE`. The planner/VDBE may lower
recognized mutations to intrinsic operations, but those intrinsics are an
implementation detail and must preserve normal transaction semantics.

This contract does not add new SQL syntax. `EXPLAIN CONCURRENCY` is deferred to
`bd-agent-swarm-coordination-transparency-8jr6u.6`; until then, diagnostics are
observable through PRAGMA output and the event table.

## Shared Rules

All three writable coordination tables follow these rules:

- Mutations run inside the caller's current transaction.
- Rollback undoes claims, lease changes, and range assignments made in that
  transaction.
- Commit publishes ownership through the same MVCC/WAL path as ordinary table
  writes.
- Concurrent conflicts are row/page conflicts surfaced as stable reason codes,
  not as file-level writer serialization.
- Expiration is checked by claim, acquire, renew, release, split, merge, and
  introspection statements. There is no background task required for expiry.
- Time inputs are integer Unix milliseconds supplied by the caller as `:now_ms`
  or by a later deterministic clock helper. Tests should prefer bound values.
- A successful mutation returns a row through `RETURNING`.
- A rejected mutation returns no mutation row and records one diagnostic event
  with a stable `reason_code`.
- Human text may evolve, but `reason_code` values are golden-testable API.

Required shared diagnostic fields:

| Field | Meaning |
|---|---|
| `trace_id` | Optional replay or live trace id. |
| `run_id` | Optional replay or harness run id. |
| `scenario_id` | Optional workload scenario id. |
| `statement_fingerprint` | Stable statement-shape fingerprint when known. |
| `plan_id` | Planner/VDBE plan id when known. |
| `worker_id` | Caller-supplied worker identity. |
| `connection_id` | Engine connection id when available. |
| `transaction_id` | Engine transaction id when available. |
| `busy_family` | `none`, `busy`, `busy_snapshot`, or `busy_recovery`. |
| `conflict_reason` | Storage or MVCC reason when known. |
| `fallback_reason` | Compatibility fallback reason when relevant. |
| `first_failure_diag` | First actionable failure string or `none`. |

## `fsqlite_queue`

`fsqlite_queue` is a durable work queue. It provides atomic claim, release,
complete, abandon, and expiration semantics for many workers.

### Columns

| Column | Type | Required | Meaning |
|---|---:|---:|---|
| `queue_name` | TEXT | yes | Logical queue namespace. |
| `item_key` | TEXT | yes | Stable item id inside `queue_name`. |
| `status` | TEXT | yes | `ready`, `claimed`, `done`, `abandoned`, or `dead_letter`. |
| `priority` | INTEGER | yes | Higher values claim first. Default `0`. |
| `available_at_ms` | INTEGER | yes | Earliest claim time. Default `0`. |
| `payload_ref` | TEXT | no | Opaque application reference; never interpreted by the engine. |
| `owner_id` | TEXT | no | Worker currently owning a claim. |
| `claim_attempt_id` | TEXT | no | Idempotency key for the latest claim attempt. |
| `claim_seq` | INTEGER | yes | Monotonic claim generation. Default `0`. |
| `claimed_at_ms` | INTEGER | no | Claim time. |
| `expires_at_ms` | INTEGER | no | Claim expiration time. |
| `completed_at_ms` | INTEGER | no | Completion time. |
| `abandoned_at_ms` | INTEGER | no | Abandon time. |
| `created_at_ms` | INTEGER | yes | Creation time. |
| `updated_at_ms` | INTEGER | yes | Last mutation time. |
| `last_reason_code` | TEXT | no | Last stable reason code for this item. |

Constraints:

- Primary key: `(queue_name, item_key)`.
- `status` must be one of the listed values.
- `claim_seq >= 0`.
- `expires_at_ms IS NULL OR expires_at_ms > claimed_at_ms`.
- `owner_id IS NOT NULL` when `status = 'claimed'`.
- `owner_id IS NULL` when `status IN ('ready', 'done', 'abandoned', 'dead_letter')`.

Required access paths:

- `(queue_name, status, priority DESC, available_at_ms, item_key)` for claims.
- `(queue_name, owner_id, status)` for worker introspection.
- `(queue_name, expires_at_ms)` for expiration scans.

### Enqueue

```sql
INSERT INTO fsqlite_queue(
    queue_name,
    item_key,
    status,
    priority,
    available_at_ms,
    payload_ref,
    created_at_ms,
    updated_at_ms
) VALUES (
    :queue_name,
    :item_key,
    'ready',
    :priority,
    :available_at_ms,
    :payload_ref,
    :now_ms,
    :now_ms
) RETURNING queue_name, item_key, status, priority, claim_seq;
```

Duplicate enqueue rejects with `queue_duplicate_item`.

### Claim

The canonical claim is a single `UPDATE ... RETURNING`. It chooses one ready
item, marks it claimed, and returns the ownership generation.

```sql
UPDATE fsqlite_queue
   SET status = 'claimed',
       owner_id = :worker_id,
       claim_attempt_id = :claim_attempt_id,
       claim_seq = claim_seq + 1,
       claimed_at_ms = :now_ms,
       expires_at_ms = :now_ms + :ttl_ms,
       updated_at_ms = :now_ms,
       last_reason_code = 'ok'
 WHERE queue_name = :queue_name
   AND item_key = (
       SELECT item_key
         FROM fsqlite_queue
        WHERE queue_name = :queue_name
          AND status = 'ready'
          AND available_at_ms <= :now_ms
        ORDER BY priority DESC, available_at_ms ASC, item_key ASC
        LIMIT 1
   )
RETURNING queue_name, item_key, owner_id, claim_attempt_id, claim_seq,
          expires_at_ms, last_reason_code;
```

If two writers race for the same ready item, at most one commit can publish the
new `claim_seq`. The loser observes `queue_claim_conflict` or a normal `Busy*`
family error with the diagnostic event carrying `busy_family`.

An idempotent retry with the same `(queue_name, item_key, claim_attempt_id,
owner_id)` may return the existing claim row if the claim has not expired.

### Release

```sql
UPDATE fsqlite_queue
   SET status = 'ready',
       owner_id = NULL,
       claim_attempt_id = NULL,
       claimed_at_ms = NULL,
       expires_at_ms = NULL,
       available_at_ms = :available_at_ms,
       updated_at_ms = :now_ms,
       last_reason_code = 'ok'
 WHERE queue_name = :queue_name
   AND item_key = :item_key
   AND status = 'claimed'
   AND owner_id = :worker_id
   AND claim_seq = :claim_seq
RETURNING queue_name, item_key, status, claim_seq, last_reason_code;
```

### Complete

```sql
UPDATE fsqlite_queue
   SET status = 'done',
       owner_id = NULL,
       completed_at_ms = :now_ms,
       updated_at_ms = :now_ms,
       last_reason_code = 'ok'
 WHERE queue_name = :queue_name
   AND item_key = :item_key
   AND status = 'claimed'
   AND owner_id = :worker_id
   AND claim_seq = :claim_seq
RETURNING queue_name, item_key, status, completed_at_ms, last_reason_code;
```

### Abandon

```sql
UPDATE fsqlite_queue
   SET status = 'abandoned',
       owner_id = NULL,
       abandoned_at_ms = :now_ms,
       updated_at_ms = :now_ms,
       last_reason_code = 'ok'
 WHERE queue_name = :queue_name
   AND item_key = :item_key
   AND status IN ('ready', 'claimed')
RETURNING queue_name, item_key, status, abandoned_at_ms, last_reason_code;
```

### Expire Claims

Expiration is a caller-visible DML operation, not a daemon:

```sql
UPDATE fsqlite_queue
   SET status = 'ready',
       owner_id = NULL,
       claim_attempt_id = NULL,
       claimed_at_ms = NULL,
       expires_at_ms = NULL,
       updated_at_ms = :now_ms,
       last_reason_code = 'queue_claim_expired'
 WHERE queue_name = :queue_name
   AND status = 'claimed'
   AND expires_at_ms <= :now_ms
RETURNING queue_name, item_key, claim_seq, last_reason_code;
```

### Queue Reason Codes

| Code | Meaning |
|---|---|
| `ok` | Mutation succeeded. |
| `queue_empty` | No ready item matched the claim filter. |
| `queue_duplicate_item` | `(queue_name, item_key)` already exists. |
| `queue_invalid_status` | Requested status transition is illegal. |
| `queue_item_not_claimed` | Release or complete targeted an unclaimed item. |
| `queue_owner_mismatch` | Owner or claim generation did not match. |
| `queue_claim_conflict` | Concurrent writer won the claim race. |
| `queue_claim_expired` | Claim was expired by an explicit expiration statement. |
| `queue_ttl_invalid` | `:ttl_ms <= 0` or expiration is not after claim time. |
| `queue_payload_too_large` | Payload reference exceeds the catalog limit. |

### Executable Slice for `.3`

`bd-agent-swarm-coordination-transparency-8jr6u.3` starts from an
ordinary-table executable shim before the built-in virtual catalog table lands.
The shim uses the queue columns, constraints, claim index, and
`UPDATE ... RETURNING` contract above on a user-created table named
`fsqlite_queue_contract`. This keeps the first proof on the live parser, VDBE,
pager, WAL, and MVCC path without inventing a temporary engine-side API.

The required same-process harness is
`crates/fsqlite-core/tests/agent_swarm_queue_claim_contract.rs`. It proves:

- Empty queues return zero mutation rows.
- Already-claimed rows are not claimed by another worker.
- Idempotent retry returns the existing matching ownership row.
- Release requires matching `owner_id` and `claim_seq`.
- Abandon clears ownership and retry state while returning the abandoned row.
- Rolling back a claim restores the row to `ready` with no owner.
- Two concurrent connections racing for one item can publish only one owner
  while `concurrent_mode_default` remains true.
- The executable trace points carry `queue_name`, `worker_id`,
  `claim_attempt_id`, `statement_fingerprint`, `conflict_reason`, and
  `elapsed_ms` fields so the virtual-table implementation has a concrete
  diagnostics contract to preserve.

This shim is not the final user surface. The final implementation must expose
`fsqlite_queue` as the built-in virtual catalog table, but it must preserve the
observable row shapes and failure classes covered by the shim. Once the virtual
table exists, the harness should be reused by swapping the setup DDL for the
built-in catalog surface rather than rewriting the behavioral assertions.

The first implementation bridge is deliberately narrow:

1. Keep claim, release, rollback, and contention tests table-backed.
2. Add the virtual table/catalog storage only after the row-shape contract is
   green on ordinary DML.
3. Preserve the same `queue_name`, `item_key`, `owner_id`,
   `claim_attempt_id`, `claim_seq`, `last_reason_code`, `statement_fingerprint`,
   `conflict_reason`, and elapsed-time fields when diagnostics are wired.
4. Treat a stale losing claim as retryable `queue_claim_conflict` or the
   existing transient `Busy*`/serialization family, never as silent success.

## `fsqlite_lease`

`fsqlite_lease` coordinates exclusive ownership of named resources. It is for
short critical sections, shard leadership, and singleton job ownership.

### Columns

| Column | Type | Required | Meaning |
|---|---:|---:|---|
| `lease_key` | TEXT | yes | Stable resource key. Primary key. |
| `owner_id` | TEXT | yes | Current lease owner. |
| `lease_token` | TEXT | yes | Opaque owner token required for renew/release. |
| `generation` | INTEGER | yes | Monotonic ownership generation. |
| `state` | TEXT | yes | `active`, `released`, or `expired`. |
| `acquired_at_ms` | INTEGER | yes | Acquisition time. |
| `renewed_at_ms` | INTEGER | yes | Last renewal time. |
| `expires_at_ms` | INTEGER | yes | Lease expiration time. |
| `metadata_ref` | TEXT | no | Opaque application reference. |
| `last_reason_code` | TEXT | no | Last stable reason code for this lease. |

Constraints:

- `generation >= 1`.
- `expires_at_ms > renewed_at_ms`.
- `state IN ('active', 'released', 'expired')`.
- Only one `active` row exists for each `lease_key`.

### Acquire

Acquire has two normal-DML paths inside the caller's transaction. A released or
expired row is transferred with `UPDATE`; a missing key is created with
`INSERT ... SELECT`. Implementations may later lower this pattern to one
planner-recognized intrinsic, but the observable row shape and reason codes stay
the same.

Take over an expired or released row:

```sql
UPDATE fsqlite_lease
   SET owner_id = :worker_id,
       lease_token = :lease_token,
       generation = generation + 1,
       state = 'active',
       acquired_at_ms = :now_ms,
       renewed_at_ms = :now_ms,
       expires_at_ms = :now_ms + :ttl_ms,
       metadata_ref = :metadata_ref,
       last_reason_code = 'ok'
 WHERE lease_key = :lease_key
   AND (state IN ('released', 'expired') OR expires_at_ms <= :now_ms)
RETURNING lease_key, owner_id, lease_token, generation, expires_at_ms,
          last_reason_code;
```

Create a missing lease key:

```sql
INSERT INTO fsqlite_lease(
    lease_key,
    owner_id,
    lease_token,
    generation,
    state,
    acquired_at_ms,
    renewed_at_ms,
    expires_at_ms,
    metadata_ref,
    last_reason_code
)
SELECT
    :lease_key,
    :worker_id,
    :lease_token,
    1,
    'active',
    :now_ms,
    :now_ms,
    :now_ms + :ttl_ms,
    :metadata_ref,
    'ok'
WHERE NOT EXISTS (
    SELECT 1
      FROM fsqlite_lease
     WHERE lease_key = :lease_key
)
RETURNING lease_key, owner_id, lease_token, generation, expires_at_ms,
          last_reason_code;
```

If the row exists and is still active for another owner, neither mutation
transfers ownership and the event reason is `lease_already_active`.

### Renew

```sql
UPDATE fsqlite_lease
   SET renewed_at_ms = :now_ms,
       expires_at_ms = :now_ms + :ttl_ms,
       last_reason_code = 'ok'
 WHERE lease_key = :lease_key
   AND owner_id = :worker_id
   AND lease_token = :lease_token
   AND generation = :generation
   AND state = 'active'
   AND expires_at_ms > :now_ms
RETURNING lease_key, owner_id, lease_token, generation, expires_at_ms,
          last_reason_code;
```

### Release

```sql
UPDATE fsqlite_lease
   SET state = 'released',
       expires_at_ms = :now_ms,
       last_reason_code = 'ok'
 WHERE lease_key = :lease_key
   AND owner_id = :worker_id
   AND lease_token = :lease_token
   AND generation = :generation
   AND state = 'active'
RETURNING lease_key, owner_id, generation, state, last_reason_code;
```

### Expire

```sql
UPDATE fsqlite_lease
   SET state = 'expired',
       last_reason_code = 'lease_expired'
 WHERE lease_key = :lease_key
   AND state = 'active'
   AND expires_at_ms <= :now_ms
RETURNING lease_key, owner_id, generation, state, last_reason_code;
```

### Lease Reason Codes

| Code | Meaning |
|---|---|
| `ok` | Mutation succeeded. |
| `lease_already_active` | Non-expired lease is owned by another token. |
| `lease_owner_mismatch` | Owner, token, or generation did not match. |
| `lease_expired` | Active lease crossed `expires_at_ms`. |
| `lease_ttl_invalid` | `:ttl_ms <= 0` or expiration is not after renewal. |
| `lease_generation_conflict` | Concurrent writer advanced the generation. |
| `lease_invalid_state` | Requested state transition is illegal. |
| `lease_metadata_too_large` | Metadata reference exceeds the catalog limit. |

### Executable Ordinary-Table Contract Slice

`bd-agent-swarm-coordination-transparency-8jr6u.4` starts from the same
ordinary-table proof strategy as the queue contract. The shim uses
`fsqlite_lease_contract` and the public `fsqlite_lease` columns, generation
rules, clock boundary rules, and reason-code vocabulary. This keeps the first
lease proof on the live parser, VDBE, MVCC, WAL, and transaction stack before a
built-in virtual catalog table is introduced.

The executable contract lives in
`crates/fsqlite-core/tests/agent_swarm_lease_contract.rs`. It proves:

- Missing-key acquire creates generation `1` and returns an active ownership
  row.
- Non-expired active ownership cannot be stolen through takeover.
- Renew, transfer, and release require matching `owner_id`, `lease_token`, and
  `generation`.
- Renew succeeds before `expires_at_ms` and fails deterministically at exactly
  `expires_at_ms`.
- Transaction-driven expiry records `lease_expired` without a background task.
- Released or expired leases can be reacquired by incrementing `generation`.
- Rollback restores the previous owner, token, generation, state, and
  expiration.
- Concurrent expired-owner takeover publishes at most one owner; stale losers
  surface as `lease_generation_conflict` or the existing retryable transient
  family.
- The executable trace points carry `lease_key`, `owner_id`, `lease_token`,
  `renew_interval_ms`, `expiration_reason`, `conflict_reason`, and
  `elapsed_ms`.

Future code that lowers this surface into a built-in `fsqlite_lease` catalog
must preserve this row-shape contract:

1. Keep acquire, renew, transfer, release, expire, rollback, and contention
   tests table-backed until the built-in table passes the same assertions.
2. Keep expiration transaction-driven; no global writer lock or background
   orphan runtime is allowed.
3. Preserve the same ownership fields and stable reason codes in diagnostics.
4. Treat a stale losing takeover as retryable `lease_generation_conflict` or
   the existing transient `Busy*`/serialization family, never as double
   ownership or silent success.

## `fsqlite_worker_ranges`

`fsqlite_worker_ranges` assigns disjoint key ranges to workers. The initial
mode is advisory. Enforced range ownership may be added only after tests prove
the advisory contract and diagnostics.

### Columns

| Column | Type | Required | Meaning |
|---|---:|---:|---|
| `range_name` | TEXT | yes | Logical allocator namespace. |
| `range_id` | TEXT | yes | Stable range id inside `range_name`. |
| `table_name` | TEXT | yes | User table covered by the range. |
| `index_name` | TEXT | no | Index or `NULL` for table rowid/key order. |
| `range_start` | TEXT | yes | Inclusive lower key encoded in stable text form. |
| `range_end` | TEXT | yes | Exclusive upper key encoded in stable text form. |
| `owner_id` | TEXT | no | Worker currently assigned. |
| `lease_token` | TEXT | no | Optional token tying range ownership to a lease. |
| `generation` | INTEGER | yes | Monotonic assignment generation. |
| `mode` | TEXT | yes | `advisory` or `enforced`. Initial implementation uses `advisory`. |
| `state` | TEXT | yes | `available`, `assigned`, `released`, or `retired`. |
| `assigned_at_ms` | INTEGER | no | Assignment time. |
| `expires_at_ms` | INTEGER | no | Optional assignment expiration. |
| `split_parent` | TEXT | no | Parent range id for split lineage. |
| `last_reason_code` | TEXT | no | Last stable reason code for this range. |

Constraints:

- Primary key: `(range_name, range_id)`.
- Ranges with the same `(range_name, table_name, index_name)` must not overlap
  unless one side is `retired`.
- `range_start < range_end` under the selected key encoding.
- `generation >= 1`.
- `mode IN ('advisory', 'enforced')`.
- `state IN ('available', 'assigned', 'released', 'retired')`.

### Allocate

```sql
UPDATE fsqlite_worker_ranges
   SET state = 'assigned',
       owner_id = :worker_id,
       lease_token = :lease_token,
       generation = generation + 1,
       assigned_at_ms = :now_ms,
       expires_at_ms = :expires_at_ms,
       last_reason_code = 'ok'
 WHERE range_name = :range_name
   AND range_id = (
       SELECT range_id
         FROM fsqlite_worker_ranges
        WHERE range_name = :range_name
          AND table_name = :table_name
          AND coalesce(index_name, '') = coalesce(:index_name, '')
          AND state IN ('available', 'released')
        ORDER BY range_start ASC, range_id ASC
        LIMIT 1
   )
RETURNING range_name, range_id, table_name, index_name, range_start,
          range_end, owner_id, generation, mode, last_reason_code;
```

### Split

Splitting retires the parent range and inserts two available child ranges in
the same transaction. Both inserts must pass the non-overlap constraint.

```sql
UPDATE fsqlite_worker_ranges
   SET state = 'retired',
       last_reason_code = 'ok'
 WHERE range_name = :range_name
   AND range_id = :range_id
   AND owner_id = :worker_id
   AND generation = :generation
RETURNING range_name, range_id, state, last_reason_code;
```

```sql
INSERT INTO fsqlite_worker_ranges(
    range_name, range_id, table_name, index_name, range_start, range_end,
    owner_id, lease_token, generation, mode, state, split_parent,
    last_reason_code
) VALUES
    (:range_name, :left_range_id, :table_name, :index_name, :range_start,
     :split_key, NULL, NULL, 1, 'advisory', 'available', :range_id, 'ok'),
    (:range_name, :right_range_id, :table_name, :index_name, :split_key,
     :range_end, NULL, NULL, 1, 'advisory', 'available', :range_id, 'ok')
RETURNING range_name, range_id, range_start, range_end, split_parent,
          last_reason_code;
```

### Merge

Merging retires two adjacent available or released ranges and inserts one
replacement range. Both parent rows must be locked by the caller's transaction.

```sql
INSERT INTO fsqlite_worker_ranges(
    range_name, range_id, table_name, index_name, range_start, range_end,
    owner_id, lease_token, generation, mode, state, split_parent,
    last_reason_code
) VALUES (
    :range_name, :merged_range_id, :table_name, :index_name, :left_start,
    :right_end, NULL, NULL, 1, 'advisory', 'available', NULL, 'ok'
) RETURNING range_name, range_id, range_start, range_end, last_reason_code;
```

The same transaction must then mark the two source ranges `retired`.

### Release

```sql
UPDATE fsqlite_worker_ranges
   SET state = 'released',
       owner_id = NULL,
       lease_token = NULL,
       expires_at_ms = NULL,
       last_reason_code = 'ok'
 WHERE range_name = :range_name
   AND range_id = :range_id
   AND owner_id = :worker_id
   AND generation = :generation
RETURNING range_name, range_id, state, generation, last_reason_code;
```

### Range Reason Codes

| Code | Meaning |
|---|---|
| `ok` | Mutation succeeded. |
| `range_exhausted` | No available range matched allocation filters. |
| `range_overlap` | Insert or merge would overlap an active range. |
| `range_gap` | Merge parents are not adjacent. |
| `range_owner_mismatch` | Owner or generation did not match. |
| `range_invalid_bounds` | `range_start >= range_end` or invalid key encoding. |
| `range_generation_conflict` | Concurrent writer advanced the generation. |
| `range_invalid_state` | Requested state transition is illegal. |
| `range_enforced_unsupported` | `mode = 'enforced'` requested before support lands. |

## Introspection

Operators inspect live ownership with ordinary `SELECT` statements:

```sql
SELECT queue_name, item_key, status, owner_id, claim_seq, expires_at_ms
  FROM fsqlite_queue
 WHERE queue_name = :queue_name
 ORDER BY priority DESC, item_key;
```

```sql
SELECT lease_key, owner_id, generation, expires_at_ms, state
  FROM fsqlite_lease
 WHERE state = 'active'
 ORDER BY expires_at_ms;
```

```sql
SELECT range_name, range_id, table_name, index_name, range_start, range_end,
       owner_id, mode, state
  FROM fsqlite_worker_ranges
 WHERE range_name = :range_name
 ORDER BY table_name, coalesce(index_name, ''), range_start;
```

## Diagnostics

### `fsqlite_coordination_events`

This read-only virtual table exposes recent coordination outcomes. It is
bounded and connection-local by default. Future production telemetry may export
aggregates, but this table is the local SQL debugging surface.

Columns:

| Column | Type | Meaning |
|---|---:|---|
| `event_seq` | INTEGER | Monotonic event sequence in the connection. |
| `event_ts_ms` | INTEGER | Event time, if known. |
| `surface` | TEXT | `queue`, `lease`, `range`, or `diagnostic`. |
| `operation` | TEXT | `enqueue`, `claim`, `release`, `complete`, `abandon`, `expire`, `acquire`, `renew`, `allocate`, `split`, `merge`, or `introspect`. |
| `reason_code` | TEXT | Stable reason code. |
| `reason_text` | TEXT | Human explanation. |
| `queue_name` | TEXT | Queue namespace, when relevant. |
| `item_key` | TEXT | Queue item key, when relevant. |
| `lease_key` | TEXT | Lease key, when relevant. |
| `range_name` | TEXT | Range namespace, when relevant. |
| `range_id` | TEXT | Range id, when relevant. |
| `worker_id` | TEXT | Caller-supplied worker id, when relevant. |
| `claim_attempt_id` | TEXT | Queue claim idempotency key, when relevant. |
| `statement_fingerprint` | TEXT | Statement-shape fingerprint, when known. |
| `plan_id` | TEXT | Planner/VDBE plan id, when known. |
| `busy_family` | TEXT | Busy classification. |
| `conflict_reason` | TEXT | MVCC/storage conflict reason. |
| `fallback_reason` | TEXT | Compatibility fallback reason. |
| `first_failure_diag` | TEXT | First actionable failure diagnostic or `none`. |

### PRAGMAs

`PRAGMA fsqlite_coordination_reason_codes;`

Returns all stable codes with columns:

```text
surface, reason_code, severity, retryable, human_text
```

`PRAGMA fsqlite_coordination_status;`

Returns one row per enabled surface:

```text
surface, enabled, writable, mode, diagnostics_available,
concurrent_mode_default_observed, supported_fast_path, fallback_reason
```

`PRAGMA fsqlite_coordination_reset;`

Clears bounded connection-local diagnostics and returns:

```text
cleared_events, previous_first_failure_diag
```

## Transaction and Crash Semantics

The writable virtual tables are transactional catalog surfaces:

- A coordination mutation is atomic with the caller's surrounding data writes.
- A worker may claim a queue item and update application tables in one
  transaction; rollback releases both.
- Lease and range ownership changes survive process restart only after commit.
- Expired claims, leases, and ranges remain visible until a caller executes an
  expiration or takeover statement.
- Crash recovery replays committed catalog changes through WAL recovery and
  discards uncommitted coordination changes.
- The engine must not spawn orphan tasks or create a runtime to expire rows.
- File-lock fallback is reported through `fallback_reason`; it is not the
  normal coordination path.

`Busy*` outcomes are retryable when the reason-code registry says so. Retrying
must use the returned generation or a fresh read; callers must not assume an old
`claim_seq`, `lease_token`, or range `generation` remains valid after a retry.

## Compatibility and Fallback Visibility

The API is compatible with existing agent-swarm replay artifacts by carrying the
fields defined in the overlap manifest: `trace_id`, `run_id`, `scenario_id`,
`statement_fingerprint`, `plan_id`, `queue_key`, `claim_attempt_id`,
`lease_key`, `range_id`, `range_start`, `range_end`, `conflict_reason`,
`busy_family`, `fallback_reason`, `artifact_path`, `artifact_hash`,
`replay_command`, `heavy_rch_command`, and `first_failure_diag`.

If an operation falls back to a slower compatibility path, the mutation may
still succeed, but diagnostics must expose:

- `supported_fast_path = false`
- `fallback_reason`
- `impact_class` when known
- `diagnostics_available = true`

Fallback must never be used to silently disable concurrent-writer mode.

### Executable Ordinary-Table Contract Slice for `.7`

`bd-agent-swarm-coordination-transparency-8jr6u.7` starts from an
ordinary-table diagnostic shim before the built-in event table and PRAGMA
surface land. The shim uses `fsqlite_fallback_reason_codes_contract` and
`fsqlite_fallback_events_contract` to pin the stable row shape for fallback
classification, impact fields, and operator aggregation. This keeps the first
proof on normal SQL DML, aggregation, rollback, and reset semantics without
adding parser syntax or editing runtime fallback dispatch as a prerequisite.

The executable contract lives in
`crates/fsqlite-core/tests/agent_swarm_fallback_transparency_contract.rs`. It
proves:

- Stable reason-code rows exist for `unsupported_sql_shape`,
  `planner_bypass`, `storage_fallback`, and `diagnostics_unavailable`.
- Supported fast-path statements keep `fallback_reason` empty while still
  exposing diagnostics availability.
- Mixed workloads aggregate fallback frequency by `statement_fingerprint`,
  `plan_id`, `table_name`, `workload_lane`, and `fallback_reason`.
- Aggregates preserve concurrency, durability, memory, latency, diagnostics,
  and first-failure diagnostic fields.
- Fallback event rows are transactional and roll back with the caller's
  transaction.
- Diagnostic reset clears the bounded event state without changing SQL
  correctness or concurrent-writer defaults.

Future code that exposes this as `fsqlite_coordination_events`, PRAGMA output,
or `EXPLAIN CONCURRENCY` rows must preserve this row-shape contract:

1. Keep stable fallback reason codes golden-testable; human text may evolve.
2. Do not count supported fast paths as compatibility fallback work.
3. Aggregate by statement, plan, table, workload lane, and reason so operators
   can identify swarm-responsive native-path gaps.
4. Preserve `supported_fast_path`, `fallback_reason`, impact fields,
   `diagnostics_available`, and `first_failure_diag`.
5. Never use fallback transparency to disable concurrent-writer mode or add a
   file-level writer bottleneck.

## Required Tests

Implementation beads must name and land tests in these categories:

- Unit tests for every valid status transition and every reason code.
- Unit tests for empty queue, duplicate enqueue, expired claim, expired lease,
  invalid TTL, invalid range bounds, overlap, split, merge, and release.
- Property or deterministic interleaving tests proving no double claim, no
  double active lease, and no overlapping active worker ranges.
- Rollback tests proving queue, lease, and range changes revert with the caller
  transaction.
- Golden tests for `PRAGMA fsqlite_coordination_reason_codes`,
  `PRAGMA fsqlite_coordination_status`, and `fsqlite_coordination_events`.
- E2E tests with at least two concurrent connections claiming disjoint and
  contended work while `concurrent_mode_default` remains true.
- Replay-lab adapter tests that add sanitized coordination fields to the
  existing agent-swarm trace artifacts without forking the replay schema.
- Heavy proof recipes must use foreground `timeout ... rch exec -- ...` command
  shapes when the run is CPU-heavy.

## Forbidden Implementation Paths

- Do not add a file-level or connection-level writer lock for coordination.
- Do not default any queue, lease, range, harness, executor, or benchmark
  setting to serialized mode.
- Do not create a background expiration task or runtime.
- Do not duplicate Agent Mail or Beads concepts inside FrankenSQLite; these are
  generic application coordination primitives.
- Do not fork replay, scorecard, or SLO governor artifacts for this feature.
- Do not publish performance claims for this API without a cited benchmark or
  replay artifact that measures the exact workload shape.

## Downstream Work Split

- `bd-agent-swarm-coordination-transparency-8jr6u.3` owns queue claim/release
  implementation and tests.
- `bd-agent-swarm-coordination-transparency-8jr6u.4` owns lease heartbeat,
  expiration, and takeover implementation.
- `bd-agent-swarm-coordination-transparency-8jr6u.5` owns worker range
  allocation, split, merge, and overlap enforcement.
- `bd-agent-swarm-coordination-transparency-8jr6u.6` owns richer contention
  diagnostics and any future `EXPLAIN CONCURRENCY` syntax.
- `bd-agent-swarm-coordination-transparency-8jr6u.7` owns compatibility-path
  fallback transparency.
- `bd-agent-swarm-coordination-transparency-8jr6u.8` owns replay-lab and SLO
  governor adapter integration.
- `bd-agent-swarm-coordination-transparency-8jr6u.9` owns the broader
  unit/property/golden regression matrix.
- `bd-agent-swarm-coordination-transparency-8jr6u.10` owns the final E2E proof
  pack and runbook.
