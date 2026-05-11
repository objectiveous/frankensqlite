# Free-page Dispatch Candidate - 2026-05-11

Purpose: evaluate a one-lever pager dispatch patch after the current DELETE CPU
screen showed `TransactionKind::free_page` in the top self-time frames.

Change under test:

- `crates/fsqlite-pager/src/traits.rs`
- `TransactionKind::free_page` now matches concrete transaction variants
  directly, mirroring the existing static dispatch for `get_page` and
  `write_page_data`.

Build and verification:

- Git: `main @ 29e8e1101139ee633a299927cd5d30e0346aba4c`, dirty with this
  source patch during measurement.
- Build: `release-perf`, opt-level 3, LTO.
- Focused compile proof: `cargo check -p fsqlite-pager --all-targets` passed
  locally with `CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-free-page-local-check`.
- Broad pager test run completed compilation but reported three failures under
  parallel `--nocapture`; each failed test passed when rerun serially:
  `test_e2e_write_overwrite_verify_latest`,
  `test_fault_drop_condvar_notify_waiters_recover_via_timeout`, and
  `test_fault_during_phase_c_returns_error_and_wal_frames_survive`.
- Final workspace gates after the last source cleanup passed:
  `cargo fmt --check`, `git diff --check`,
  `cargo check --workspace --all-targets`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Final serial pager test proof passed:
  `cargo test -p fsqlite-pager -- --test-threads=1` reported 579 executed
  tests passed and 10 ignored.
- `ubs` on the touched code/docs still exits nonzero on the existing
  `traits.rs` panic/unwrap/assert/direct-indexing inventory, but its
  fmt/clippy/check/test-build/audit/deny subchecks were clean. The final source
  patch does not add a new direct `panic!` arm for `free_page`; the impossible
  `Drained` sentinel path routes through the existing centralized helper.

Focused DELETE probes:

| Probe | FSQLite | C SQLite | F/C ratio |
| --- | ---: | ---: | ---: |
| 10K/500 delete isolated, run 1 | 355 ns/delete | 280 ns/delete | 1.27x |
| 10K/500 delete isolated, run 2 | 349 ns/delete | 279 ns/delete | 1.25x |
| 10K/500 delete standard, run 1 | 561 ns/delete | 324 ns/delete | 1.73x |
| 10K/500 delete standard, run 2 | 572 ns/delete | 338 ns/delete | 1.69x |

Full quick matrix:

| Metric | Previous source artifact | Candidate |
| --- | ---: | ---: |
| FSQLite faster / comparable / C SQLite faster | 79 / 1 / 13 | 79 / 0 / 14 |
| Geomean F/C | 0.2797617872 | 0.2732153924 |
| Median F/C | 0.2796272982 | 0.2869799908 |
| Average F/C | 0.5226487663 | 0.5121668939 |
| p90 F/C | 1.1773280477 | 1.1048789446 |
| p99 F/C | 3.6542783060 | 3.3426258993 |
| Weighted score | 0.3846803250 | 0.3765660231 |
| write_single geomean | 1.2845404564 | 1.2586610602 |

DELETE row movement in the full quick matrix:

| Scenario | Previous F ms | Candidate F ms | Previous F/C | Candidate F/C |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 0.008456 | 0.007434 | 3.6542783060 | 3.3426258993 |
| 1000 rows / delete 50 rows | 0.033914 | 0.032380 | 2.1222778473 | 2.0612387803 |
| 10000 rows / delete 500 rows | 0.301384 | 0.302406 | 1.8445004774 | 1.8658629136 |

Decision: keep. The candidate improves the primary weighted score, geomean,
average, p90, p99, and write-single geomean. The largest remaining DELETE row
still needs the broader transaction-local mutation primitive tracked in the
performance ledger, but this patch removes one measured dispatch cost without
changing transaction semantics.
