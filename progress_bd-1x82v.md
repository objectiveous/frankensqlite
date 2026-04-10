# bd-1x82v progress

## 2026-04-10

- Audited the existing `PRAGMA integrity_check` / `PRAGMA quick_check` lock-byte-page invariant in `fsqlite-core` and confirmed the guard is already present in the integrity walker.
- Added a regression test that corrupts an interior table root to point at the reserved lock-byte page and asserts that `PRAGMA quick_check` returns a diagnostic instead of `"ok"`.
- `wal_checkpoint` code path was re-audited while tracing the bead; no functional change was needed in this slice.
