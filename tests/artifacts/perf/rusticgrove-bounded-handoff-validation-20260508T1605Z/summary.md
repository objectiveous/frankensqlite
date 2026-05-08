# Bounded Handoff Validation - 2026-05-08

## Source State

- Project: `/data/projects/frankensqlite`
- Final verification HEAD: `837a13915f1fd5098572a0a79d1524a6344848b6`
- Final worktree status before artifact write: `## main...origin/main` plus this artifact and the harness fix
- Bead focus: `bd-db300.2.4.4` bounded handoff validation, including contention storms and BUSY-family boundaries

## Initial Observation

The first lock-contention verification run failed in `test_lock_convoy`:

```text
hard failures: ["slow UPDATE id=1: database is busy (snapshot conflict on pages: 2)"]
```

The same focused convoy test reproduced on clean `837a1391` before the fix:

```text
cargo test -p fsqlite-harness --test bd_3plop_4_lock_contention_storms test_lock_convoy -- --nocapture --test-threads=1
```

Result: failed with the same slow-writer transient snapshot conflict.

## Fix

File changed:

- `crates/fsqlite-harness/tests/bd_3plop_4_lock_contention_storms.rs`

The slow convoy writer already treated transient `COMMIT` conflicts as an expected MVCC outcome. The failing path was a transient conflict surfacing earlier during the slow writer's `UPDATE` loop after fast writers advanced the page snapshot. The harness now handles that transient DML conflict by rolling back the slow transaction and returning without recording a hard failure. Non-transient slow-writer update errors still fail the test.

No engine code or concurrent-writer default was changed.

## Post-Fix Verification

Focused convoy replay:

```text
cargo test -p fsqlite-harness --test bd_3plop_4_lock_contention_storms test_lock_convoy -- --nocapture --test-threads=1
```

Result:

```text
test test_lock_convoy ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out
```

Full contention-storm script:

```text
bash scripts/verify_bd_3plop_4_lock_contention_storms.sh
```

Result:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
=== bd-3plop.4: All lock contention storm verification gates passed ===
```

Busy semantics matrix:

```text
cargo test -p fsqlite-e2e --test bd_2yqp6_6_2_busy_retry_error_semantics_matrix -- --nocapture --test-threads=1
```

Result:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The matrix also revalidated the concurrent-mode guard:

```text
"concurrent_mode_guard":{"default_on":true,"begin_promotes_to_concurrent":true,"pragma_off_disables_promotion":true,"pragma_on_restores_default":true}
```

## Interpretation

The observed failure was a harness classification bug: a transient MVCC snapshot conflict is an expected convoy outcome whether it reaches the slow writer at `UPDATE` time or at `COMMIT` time. After the correction, the contention-storm suite and BUSY-family matrix both pass from the clean source state.
