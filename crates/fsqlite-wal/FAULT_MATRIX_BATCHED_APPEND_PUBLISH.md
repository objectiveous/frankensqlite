# Fault Matrix: Batched Append & Publish Paths

> **Bead:** bd-db300.7.2.1 / bd-966ja
> **Date:** 2026-03-22
> **Author:** Claude Opus 4.6 performance-correctness agent
> **Scope:** C1 (batched WAL append), C2 (checksum/serialization outside publish),
>   checkpoint, and the bd-zna34 EBADF failure path.

## Path Inventory

| ID   | Path Name                     | Entry Point (file:line)                                      | Hot? |
|------|-------------------------------|--------------------------------------------------------------|------|
| P1   | Group-commit flusher          | `pager.rs:3560` `SubmitOutcome::Flusher`                     | Yes  |
| P2   | Group-commit waiter           | `pager.rs:3803` `SubmitOutcome::Waiter`                      | Yes  |
| P3   | WAL append_frames             | `wal.rs:1001` `WalFile::append_frames`                       | Yes  |
| P4   | WAL sync                      | `wal.rs:1132` `WalFile::sync`                                | Yes  |
| P5   | Checkpoint backfill           | `checkpoint_executor.rs:104`                                 | No   |
| P6   | Checkpoint WAL reset/truncate | `wal.rs:1141` `WalFile::reset`                               | No   |
| P7   | Epoch pipelining              | `pager.rs:3547-3556` next_epoch_batches submit during FLUSHING | Yes  |
| P8   | WalBackendAdapter publish     | `wal_adapter.rs:830-868` `record_appended_frames` + FEC      | Yes  |
| P9   | Phase C publish               | `pager.rs:4446-4509` commit_seq update + snapshot publish     | Yes  |
| P10  | VFS file open (IoUring+Unix)  | `uring.rs:632-672` double-fd open                            | Startup |
| P11  | VFS inode table close         | `unix.rs:1555-1581` refcount decrement + maybe_remove        | Close |

---

## Fault Cases

### F1. Crash during WAL frame write (P3)

**Trigger:** Power loss / process kill between `append_prepared_frame_bytes()` (wal.rs:1045)
and `sync()` (wal.rs:1132).

**Symptom:** Partial frame(s) on disk. Running checksum chain is broken at the torn frame.

**Recovery contract:** `WalFile::open()` (wal.rs:498-600) re-scans frames, terminates the
valid chain at the last frame whose checksum matches the running chain. Partial/torn
frames are discarded. This is the standard SQLite WAL recovery model.

**Injection hook:** `fault_inject_after_wal_append` — insert between wal.rs:1045 and the
caller's sync(). Return `Err(FrankenError::Io(...))` to simulate crash before sync.

**Proof obligation:** After injected failure, re-opening the WAL must produce
`frame_count <= pre-crash frame_count` and the checksum chain must be valid.

---

### F2. Crash during WAL sync (P4)

**Trigger:** Process kill during `file.sync(cx, flags)` (wal.rs:1133).

**Symptom:** fsync returns but some sectors may not have reached stable storage (depends
on hardware). Frames are either fully durable or fully lost (sector-atomic assumption).

**Recovery contract:** Same as F1 — checksum chain scan discards any incomplete frames.

**Injection hook:** `fault_inject_during_wal_sync` — drop the sync call and simulate
process restart.

**Proof obligation:** Re-opened WAL has consistent checksum chain. No phantom frames
survive that weren't fully written.

---

### F3. Crash between group-commit flush completion and epoch notification (P1→P7)

**Trigger:** Process kill after WAL sync completes (pager.rs:3685) but before
`publish_completed_epoch()` (pager.rs:3771) notifies waiters.

**Symptom:** WAL frames are durable on disk. Waiters never received notification. On
restart, the frames are recoverable via WAL scan, but in-memory state (commit_seq,
snapshot plane) was never updated.

**Recovery contract:** On restart, `WalFile::open()` rediscovers all durable frames.
`reload_memdb_from_pager()` rebuilds state from the durable WAL + db file.

**Injection hook:** `fault_inject_after_flush_before_publish` — insert between
pager.rs:3685 (sync) and pager.rs:3761 (complete_flush). Kill process.

**Proof obligation:** After restart, all committed frames from the completed flush
are visible. Waiter transactions that were "in-flight" are rolled back (they never
committed — the group-commit's frames are durable but the waiters' phase-C
publication never happened).

**Subtlety:** With epoch pipelining (P7), `next_epoch_batches` may contain frames
submitted during the flush. These are NOT durable (they were never written to the WAL)
and must be silently dropped on restart.

---

### F4. Crash during Phase C publication (P9)

**Trigger:** Process kill between inner.lock() re-acquisition (pager.rs:4446) and
snapshot plane publication (pager.rs:4509).

**Symptom:** WAL frames are durable. `commit_seq` was incremented in-memory but the
published snapshot may not reflect it.

**Recovery contract:** On restart, WAL recovery re-establishes the correct state.
The `commit_seq` is derived from the durable frame chain, not from in-memory counters.

**Injection hook:** `fault_inject_during_phase_c` — insert between commit_seq
update and snapshot plane publication.

**Proof obligation:** After restart, commit_seq matches the number of committed
transactions recoverable from WAL. No "ghost" commits visible.

---

### F5. Flusher panics or returns error mid-batch (P1)

**Trigger:** I/O error during `wal.append_frames()` (pager.rs:3677) or
`inner.db_file.lock(cx, LockLevel::Exclusive)` (pager.rs:3670) fails.

**Symptom:** The flusher records the failure in `failed_epochs` (pager.rs,
`GroupCommitQueue::failed_epochs`) and notifies waiters. Waiter threads receive
the error and propagate it to their callers.

**Current code path:** pager.rs:3708-3724 retry loop with exponential backoff
for Busy errors. After `MAX_FLUSH_RETRIES` (10), the error propagates.

**Injection hook:** `fault_inject_wal_append_error` — make `wal.append_frames()`
return `Err(FrankenError::Busy)` N times, then succeed or fail permanently.

**Proof obligations:**
1. Waiter threads receive the same error (not a stale success).
2. No partial frames written to WAL (append_frames must be atomic-or-nothing).
3. The Condvar waiters are all woken (no stuck threads).
4. Subsequent flushes can succeed (queue is not poisoned).

---

### F6. Waiter epoch mismatch / stale epoch (P2)

**Trigger:** Race between epoch increment (consolidator.begin_flush → epoch++)
and waiter's epoch capture (pager.rs:3554 `consolidator.epoch()`).

**Symptom:** Waiter captures epoch N, but the flusher has already advanced to N+1.
Waiter then waits for epoch N+1 which may have already completed.

**Current guard:** `wait_for_epoch_outcome()` (pager.rs:3819) checks
`completed_epoch >= target_epoch` before blocking. If the epoch already
completed, it returns immediately.

**Injection hook:** `fault_inject_epoch_skip` — artificially increment epoch
by 2 instead of 1 to test the `>=` guard.

**Proof obligation:** No waiter thread hangs indefinitely. Every waiter
either gets success or a propagated error.

---

### F7. Checkpoint WAL truncation racing with concurrent append (P5+P6 vs P3)

**Trigger:** Checkpoint calls `wal.reset(truncate=true)` (wal.rs:1159-1164) while
a concurrent writer's group-commit is preparing to append frames.

**Current guard:** The flusher acquires `Exclusive` file lock (pager.rs:3670)
before WAL I/O, and checkpoint also acquires an exclusive lock. Only one can
hold it at a time.

**Race window:** Between the flusher releasing Exclusive→Reserved (pager.rs:3698)
and the checkpoint acquiring Exclusive. If the checkpoint truncates the WAL, any
subsequent reader that tries to read "committed" frames from the truncated region
gets garbage or EOF.

**Injection hook:** `fault_inject_checkpoint_between_unlock_and_next_append` —
schedule checkpoint truncation in the window between flusher's unlock and the
next commit's lock acquisition.

**Proof obligations:**
1. Readers that opened before truncation see a consistent snapshot.
2. Writers that begin after truncation get a fresh WAL with valid header.
3. No phantom reads of post-truncation garbage.

---

### F8. **bd-zna34: Bad file descriptor (EBADF) during persistent benchmark commit** (P10+P11+P4)

**Error signature:** `COMMIT failed: Io(Os { code: 9, kind: Uncategorized, message: "Bad file descriptor" })`

**Context:** Multi-thread persistent benchmark. Each thread opens its own
`Connection::open(&path)` which creates:
- `IoUringVfs::open()` (uring.rs:632-672): opens TWO fd's per file
  - `UnixFile` via `self.unix.open()` → shared through global inode table `Arc<File>`
  - `AsupersyncIoUringFile` via `open_asupersync_backend()` → independent fd

**Root cause hypotheses (ranked by likelihood):**

**H1 (HIGH): Inode table premature removal under concurrent close/re-open.**
When a connection closes its WAL file handle (e.g., during checkpoint or
connection drop), `UnixFile::close()` (unix.rs:1555) decrements `n_ref` and
calls `maybe_remove()` (unix.rs:1574→406). If `n_ref` reaches 0, the
`InodeInfo` is removed from the global table. A subsequent `open()` for the
same path creates a NEW `InodeInfo` with a NEW `Arc<File>` — but any surviving
`Arc<File>` clones from the OLD `InodeInfo` still reference the old fd. If the
old fd gets closed (Arc refcount → 0 when the last old-generation handle drops),
the NEW generation's fd is unaffected... UNLESS:
- The `NamedTempFile` (`_tmp`) in the benchmark's setup closure gets dropped
  between Criterion iterations, deleting the underlying file
- A new iteration creates a new temp file at a DIFFERENT path
- But the global inode table or group commit queue retains stale references

**H2 (MEDIUM): NamedTempFile deletion race.**
Criterion's `iter_batched` calls the setup closure to get `(tmp, path, ...)`,
then the measurement closure takes ownership. After measurement, `tmp` drops →
OS deletes the file. If the benchmark spawns threads that haven't fully closed
their connections before `tmp` drops, those threads' WAL sync operations hit
a deleted file. The `for h in handles { h.join() }` should prevent this, but
if a thread panics (e.g., from a prior error) and `join()` returns `Err`, the
remaining threads may still be running when `tmp` drops.

**H3 (MEDIUM): AsupersyncIoUringFile fd lifetime vs IoUringFile.inner close.**
`IoUringFile::close()` delegates to `self.inner.close(cx)` (UnixFile close).
But the `asupersync_backend` field is only dropped when the `IoUringFile` is
dropped (Rust destructor order). If the asupersync backend holds an fd that
references the same file, and the UnixFile close triggers POSIX lock release
that invalidates the asupersync fd's position, subsequent operations on the
asupersync fd could fail.

**H4 (LOW): Global GroupCommitQueue outlives file handles.**
`GROUP_COMMIT_QUEUES` is a `static OnceLock<Mutex<HashMap<PathBuf, GroupCommitQueueRef>>>`.
The queue for a temp file path persists after the file is deleted. If a new
benchmark iteration reuses a path that maps to a stale queue, the flusher
may try to write to a WAL backend whose underlying file has been closed.

**H5 (LOW-MEDIUM): Auto-checkpoint WAL reset invalidating concurrent WAL state.**
`maybe_run_adaptive_autocheckpoint()` (connection.rs:11080) runs after each commit.
It calls `pager.checkpoint(&cx, mode)` which can reset or truncate the WAL via
`WalFile::reset()` (wal.rs:1141). If Thread A's auto-checkpoint resets/truncates
the shared WAL file while Thread B's WAL backend is still referencing the old
state, Thread B's next WAL sync could fail. However, the checkpoint acquires
an Exclusive file lock (checkpoint_executor.rs:104-158), and WAL append also
acquires Exclusive (pager.rs:3670), so they shouldn't overlap. The race window
would have to be between lock release and re-acquisition across threads.

**Recommended investigation steps:**
1. Add `tracing::warn!` in `UnixFile::close()` logging the fd number, path, and n_ref.
2. Add `tracing::warn!` in `maybe_remove()` when n_ref reaches 0, logging the inode key.
3. Add guard in `GroupCommitQueue` to detect stale entries (e.g., check if the
   underlying file is still valid before flushing).
4. Build a focused repro: 2 threads, single iteration, with RUST_LOG=debug tracing.
5. Check whether auto-checkpoint runs during the benchmark by adding tracing to
   `maybe_run_adaptive_autocheckpoint()` and watching for interleaving with commits.

**Injection hook:** `fault_inject_close_wal_before_commit` — close one thread's WAL
file handle immediately before another thread's commit attempts sync.

---

### F9. Checksum chain corruption from concurrent WAL header rewrite (P6 vs P3)

**Trigger:** `WalFile::reset()` (wal.rs:1157) writes a new WAL header with new
salts. A concurrent reader scanning the WAL sees the new salts for the header
but old salts in pre-existing frames.

**Current guard:** The checksum scan in `WalFile::open()` uses the salts from
the header. If frames have different salts, they're treated as the end of the
valid chain (wal.rs:527-597, salt mismatch terminates scan).

**Race window:** Between writing the new header and truncating old frames.
If the process crashes here, the old frames have old salts, the new header
has new salts. Recovery correctly ignores the old frames.

**Injection hook:** `fault_inject_crash_between_header_write_and_truncate` —
inject between wal.rs:1157 (header write) and wal.rs:1159 (truncate).

**Proof obligation:** Recovery produces zero frames (all old frames have
mismatched salts vs new header). Database is in the state of the last
checkpoint.

---

### F10. FEC encoding failure during publish (P8)

**Trigger:** FEC commit hook in `WalBackendAdapter::append_frames()`
(wal_adapter.rs:837-864) fails after frames are already appended to WAL.

**Symptom:** WAL frames are durable, but FEC repair symbols are not
generated. Self-healing capability is degraded but data is not lost.

**Current code path:** FEC hook failure is logged but does not roll back
the WAL append (the frames are already durable).

**Injection hook:** `fault_inject_fec_hook_failure` — make the FEC hook
return an error after WAL append succeeds.

**Proof obligation:** Data integrity is preserved even without FEC symbols.
The WAL can be recovered using standard checksum-chain scanning.

---

### F11. Deadlock in group-commit queue under thread starvation (P1+P2)

**Trigger:** The flusher thread holds the consolidator lock and the inner
lock simultaneously (impossible in current design — they're acquired
sequentially with releases between). But if a Condvar wait (pager.rs:3812)
is not properly notified, waiter threads hang indefinitely.

**Current guard:** `publish_completed_epoch()` calls `condvar.notify_all()`.
Additionally, `wait_for_epoch_outcome()` uses a timed wait with 100ms timeout
to prevent permanent hangs.

**Injection hook:** `fault_inject_drop_condvar_notify` — suppress the
`notify_all()` call to verify that the timeout-based recovery works.

**Proof obligation:** All waiter threads eventually unblock (via timeout
or notification). No permanent hangs.

---

## Injection Hook Summary

| Hook ID | Fault Case | Code Location | Technique |
|---------|-----------|---------------|-----------|
| `fault_inject_after_wal_append` | F1 | wal.rs:1045→caller | Return Err before sync |
| `fault_inject_during_wal_sync` | F2 | wal.rs:1133 | Drop sync, simulate restart |
| `fault_inject_after_flush_before_publish` | F3 | pager.rs:3685→3761 | Kill process |
| `fault_inject_during_phase_c` | F4 | pager.rs:4446→4509 | Kill between update+publish |
| `fault_inject_wal_append_error` | F5 | pager.rs:3677 | Inject Busy/Io error |
| `fault_inject_epoch_skip` | F6 | consolidator epoch | Increment by 2 |
| `fault_inject_checkpoint_truncate_race` | F7 | between unlock→lock | Schedule checkpoint |
| `fault_inject_close_wal_before_commit` | F8/zna34 | unix.rs:1555 | Close WAL fd pre-commit |
| `fault_inject_crash_header_truncate` | F9 | wal.rs:1157→1159 | Crash between ops |
| `fault_inject_fec_hook_failure` | F10 | wal_adapter.rs:837 | Error after append |
| `fault_inject_drop_condvar_notify` | F11 | condvar.notify_all | Suppress notify |

---

## Cross-References

- **bd-zna34** (P0 bug): Covered by F8. Root cause investigation ongoing.
- **bd-db300.3.1** (C1: batched WAL append): Covered by F1, F2, F3, F5.
- **bd-db300.3.2** (C2: checksum outside publish): Covered by F3, F4, F8.
- **bd-db300.5.3.1** (metadata publication): F3, F4 are the publish-path faults.
- **Future C5 work** (advanced group commit): F6, F7, F11 are scaling faults.

## Assumptions Ledger

1. **Sector-atomic writes:** WAL frames are assumed to be sector-aligned and
   each sector write is atomic. If torn at sub-sector granularity, the checksum
   chain catches it on recovery.

2. **fsync durability:** After `file.sync()` returns `Ok(())`, frames are on
   stable storage. If the hardware lies (write cache without battery backup),
   all bets are off — this is outside our fault model.

3. **POSIX fd semantics:** Closing an fd does not affect other fds for the same
   file opened via `open()`. The `Arc<File>` model in unix.rs depends on this.

4. **Single-process model:** The current fault matrix assumes all writers are
   in the same process (sharing the global inode table and group commit queues).
   Multi-process MVCC (bd-2l5jk) will require a separate fault matrix.

5. **No filesystem-level corruption:** We assume the filesystem does not silently
   corrupt data that was successfully fsync'd. Bit-rot detection is handled by
   the RaptorQ/FEC layer, not the fault injection layer.
