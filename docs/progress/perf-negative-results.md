# Performance Negative Results Ledger

This ledger records performance ideas that were measured and rejected. Check it
before starting a new optimization pass, and add an entry whenever a candidate is
abandoned, reverted, or kept out of the tree because the benchmark matrix did not
move in the intended direction.

Each entry should include:
- Target workload rows or benchmark section.
- Files or subsystem touched.
- Baseline and candidate evidence.
- Result and reason for rejection.
- Conditions under which the idea is worth retrying.

## 2026-05-08 - Direct INSERT concat record-body encoder

- Target: `comprehensive-bench --quick --filter insert`, especially the
  remaining `large_10col` 10K rows where the current profile still attributes
  several milliseconds to prepared direct INSERT row building.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  the source diff was manually restored after the insert matrix rejected the
  candidate.
- Candidate shape: add a `TextConcat` prepared direct-record value so
  `try_serialize_prepared_direct_simple_insert_record` could compute concat
  payload length first, then serialize concat text directly into the SQLite
  record body. The existing `text_scratch` materialization path remained the
  fallback for unsupported/lossy blob concat values and non-text affinity
  coercion. This was the broader direct-serialization retry condition from the
  prior param-one text-cache and concat-specialization rejects, not another
  cache-only variant.
- Correctness/build proof before rejection:
  `cargo fmt -p fsqlite-core --check` passed after the candidate was restored;
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-direct-concat-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`
  passed 3 targeted concat-chain tests on the candidate; and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-direct-concat-bench-target CARGO_BUILD_JOBS=16 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
  passed.
- Evidence artifacts:
  `tests/artifacts/perf/direct-concat-candidate-silveranchor-20260508T1454Z/summary.md`,
  `candidate-insert.json`, and `stdout/`.
- Result: rejected and not applied. Against the current insert-filter artifact
  `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/insert-profile.json`,
  aggregate INSERT average/geomean/p90/p99 ratios worsened from
  `0.803142 / 0.780274 / 1.074184 / 1.132336` to
  `0.857085 / 0.823145 / 1.176456 / 1.264681`. The frontier
  `large_10col` rows regressed too: single-txn 10K ratio
  `1.023510 -> 1.135677`, and record-size 10K ratio
  `1.083482 -> 1.176456`. Small tail rows also moved the wrong way, including
  `tiny_1col` single-txn 100 (`1.063987 -> 1.264681`) and
  small_3col 100-row batched (`1.132336 -> 1.189669`).
- Do not retry a direct concat length-pass/body-append encoder as a standalone
  prepared INSERT optimization. Reconsider only as part of a true whole-row
  template/page builder that reduces row construction and page-run costs
  together while improving INSERT geomean and p99 in the same A/B window.

## 2026-05-08 - WAL prepared transform coefficient precompute

- Target: low-thread concurrent-writer gap where
  `WalChecksumTransform::for_wal_frame` remained visible in the clean
  `mt-mvcc-bench` profile.
- Touched during rejected scratch candidate:
  `crates/fsqlite-wal/src/checksum.rs` and
  `crates/fsqlite-wal/src/wal.rs`; source was only edited in detached scratch
  worktree `/data/tmp/frankensqlite-windyibis-wal-pipeline-638e93f9` and was
  not applied to `main`.
- Candidate shape: precompute the WAL frame header and page-payload affine
  checksum coefficients once per prepared WAL batch, then build the prepared
  frame transform from the serialized first 8 header bytes plus the original
  page payload slice instead of calling `WalChecksumTransform::for_wal_frame`
  on the freshly copied frame bytes for every frame.
- Correctness proof on the scratch candidate:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-wal-pipeline-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-wal checksum_transform -- --nocapture`
  passed 3 checksum-transform tests. `rch` failed open to local execution
  because the scratch worktree was outside the `/data/projects` canonical
  remote root.
- Evidence artifacts:
  `tests/artifacts/perf/windyibis-wal-pipeline-precompute-20260508T102216Z/summary.md`,
  `baseline-mt-mvcc-2t.json`, `candidate-mt-mvcc-2t.json`,
  `baseline-concurrent-quick.json`, `candidate-concurrent-quick.json`, and
  `candidate.diff`.
- Result: rejected by the concurrent quick matrix. The standalone
  `mt-mvcc-bench --threads=2 --iters=10` row improved in the same-window A/B
  (`3.75 ms -> 3.35 ms` FSQLite p50 and time ratio
  `1.5307x -> 1.3217x`), but `comprehensive-bench --quick --filter concurrent`
  worsened the primary 2-writer row (`13.379 ms -> 13.638 ms`, ratio
  `1.1312x -> 1.1521x`), worsened 4 writers (`19.410 ms -> 19.734 ms`), and
  worsened aggregate concurrent average/geomean ratios
  `0.825150 / 0.744574 -> 0.845684 / 0.759243`.
- Do not retry per-batch WAL checksum coefficient precompute plus
  source-payload prepared-transform construction as a standalone optimization.
  Revisit WAL frame preparation only with a larger pipeline change that wins
  `comprehensive-bench --quick --filter concurrent` and then the full quick
  matrix in the same A/B window.

## 2026-05-08 - Direct REAL UPDATE numeric assignment shortcut

- Target: focused `UPDATE/DELETEThroughput` fixed-width REAL direct UPDATE
  rows, especially `UPDATE bench SET value = ?2 WHERE id = ?1` in the
  100-row tail.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored after the repeat focused gate rejected the change.
- Candidate shape: add a specialized
  `prepared_direct_simple_update_assignment_real_value` helper for the
  fixed-width REAL patch path so already-numeric RHS values (`Float` and
  `Integer`) skip the generic `SqliteValue` clone plus `apply_affinity` route.
  Nullable/non-REAL fallbacks, `NOT NULL` enforcement, DELETE, page I/O
  selection, and concurrent-writer defaults were left unchanged.
- Correctness proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-real-assign-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture --test-threads=1`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-real-assign-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture --test-threads=1`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/boldlion-dml-setup-profile-20260508T0908Z/summary.md`,
  `candidate-real-assignment-dml.json`,
  `candidate-real-assignment-dml-repeat2.json`, and
  `candidate-real-assignment-perf-update-100-long.err`.
- Result: rejected and restored. The isolated mutation proof improved
  `perf-update-delete 100 20000 update fsqlite isolated` from
  `656ns/update` to `624ns/update`, but the focused matrix did not meet the
  keep gate. Baseline focused DML average/geomean were
  `1.0830375533` / `1.0667483392`. The first candidate run was
  `1.0896454667` / `1.0663675452`, with p90/p99 `1.4364985741` and the
  1000-row update row at `1.1296203196x` with `17.6%` FSQLite CV. The repeat
  worsened to `1.1042726276` / `1.0843345531`, p90/p99 `1.4356310752`, and
  the 10000-row update row regressed to `1.0461834268x`.
- Do not retry numeric-only direct REAL assignment specialization as a
  standalone optimization. Reconsider only if a broader DML batch/run operator
  removes larger per-row mutation work and still wins repeated focused
  UPDATE/DELETE gates plus the full quick matrix.

## 2026-05-08 - Direct UPDATE lazy row-scratch borrow

- Target: remaining focused `UPDATE/DELETEThroughput` tail, especially the
  fixed-width REAL direct UPDATE rows in
  `UPDATE bench SET value = ?2 WHERE id = ?1`.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored after the repeat focused gate rejected the change.
- Candidate shape: delay borrowing `prepared_direct_update_row_scratch` until
  after `try_execute_prepared_direct_simple_update_fixed_width_real` declines,
  removing one `RefCell` borrow from the benchmark's fixed-width REAL direct
  UPDATE hot path while leaving DELETE, page I/O selection, cursor retention,
  and concurrent-writer defaults unchanged.
- Correctness proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-dml-current-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_all_non_ipk_columns_skips_old_payload_decode -- --nocapture --test-threads=1`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-dml-current-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture --test-threads=1`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-dml-current-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture --test-threads=1`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/boldlion-dml-current-20260508T0900Z/summary.md`,
  `head-dml-profile.json`, `candidate-lazy-update-scratch.json`, and
  `candidate-lazy-update-scratch-repeat.json`.
- Result: rejected and restored. The first dirty candidate run looked
  promising, moving focused DML average/geomean
  `1.1203914638` / `1.1122084670` to
  `1.0344792667` / `1.0225809144`, but the immediate repeat failed the
  focused gate: average/geomean worsened to
  `1.1957309163` / `1.1693177146`, p90/p99 worsened to
  `1.6946494052`, the 100-row update row was `1.3924088986x`, and the
  100-row delete row was `1.6946494052x`.
- Do not retry lazy borrowing of the direct UPDATE row-value scratch as a
  standalone optimization. Reconsider only if it falls out naturally inside a
  broader DML run operator that removes larger per-row admission or mutation
  work and wins repeated focused UPDATE/DELETE gates.

## 2026-05-08 - Private-memory direct UPDATE/DELETE `SharedTxnPageIo` bypass

- Target: remaining setup-heavy `UPDATE/DELETEThroughput` rows for private
  `:memory:` benchmark databases, especially the 100-row update/delete tail.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
  The diff was already present in the shared worktree before measurement and
  was left unstaged/uncommitted by BoldLion after rejection.
- Candidate shape: add a private-memory-only
  `direct_update_delete_page_io_context()` helper that returns `None` for
  `self.path == ":memory:" && self.pager.is_memory()`, routing prepared direct
  UPDATE/DELETE through the plain active `TransactionKind` cursor instead of
  constructing `SharedTxnPageIo`. File-backed and non-private memory databases
  still use `concurrent_page_io_context()`.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-private-dml-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_entry_proof_no_publication_for_memory_update_delete -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-private-dml-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/boldlion-private-dml-pageio-20260508T0820Z/summary.md`,
  `candidate-update.json`, and `candidate-update-repeat.json`.
- Result: rejected. First focused UPDATE/DELETE run improved the section
  geomean versus the published focused DML baseline (`1.0564291964 ->
  0.9733515023`) but regressed the 100-row delete tail to `1.7454228976x`
  with `52.8%` FSQLITE CV. The immediate repeat rejected the candidate more
  clearly: average/geomean were `1.1308286150`/`1.1042288976`, p90/p99
  `1.6349627785`, 100-row update `1.3087941304x`, 100-row delete
  `1.6349627785x`, and the larger 10K rows fell back to parity rather than a
  durable win.
- Do not retry private-memory direct UPDATE/DELETE `SharedTxnPageIo` bypass as
  a standalone optimization. Reconsider only as part of a broader batch/leaf-run
  DML operator that reduces fixed setup and mutation work together, and require
  repeated focused UPDATE/DELETE gates plus a full quick matrix where the
  100-row update/delete tails both improve.

## 2026-05-08 - Prepared direct INSERT indexed schema lookup

- Target: fixed prepared direct INSERT setup cost in the remaining 100-row
  INSERT rows and the UPDATE/DELETE setup phase that prepopulates through the
  same direct INSERT path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored after measurement.
- Candidate shape: in `prepared_direct_simple_insert_plan`, replace the
  per-prepare `schema.iter().find(|table| table.name.eq_ignore_ascii_case(...))`
  scan with the existing `schema_index_of(...)` side-index lookup, leaving all
  direct INSERT eligibility checks unchanged.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-schema-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_prepared_insert_precomputes_direct_simple_insert_plan -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/boldlion-schema-lookup-20260508T0736Z/summary.md`,
  `candidate-insert.json`, and `candidate-update.json`.
- Result: rejected and reverted. The focused INSERT filter was mixed versus the
  prior post-pagebuf artifact: average/geomean improved
  `0.8442288062 -> 0.8281973547` and `0.8145695151 -> 0.7982037686`, but
  p90/p99 worsened `1.1272106776 -> 1.1662110161` and
  `1.2931548041 -> 1.3405968544`. The focused UPDATE/DELETE gate rejected the
  candidate outright: the 100-row delete row worsened to `1.6681464056x`, the
  100-row update row was `1.4013856256x`, and DML p99 was worse than the current
  full quick tail (`1.6681464056` vs `1.4337080362`).
- Do not retry prepared direct INSERT schema side-index lookup as a standalone
  optimization. Reconsider only if it is absorbed into a broader prepared
  statement setup redesign that proves lower 100-row DML tails and better p90/p99
  in a same-window full quick matrix.

## 2026-05-08 - Prepared direct DML root `PageNumber` predecode

- Target: prepared direct DML fixed ceremony in the remaining INSERT and
  UPDATE/DELETE rows, especially small direct INSERT rows and the
  `UPDATE/DELETEThroughput` 100-row setup-heavy rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored by the reservation holder after measurement.
- Candidate shape: keep the legacy `root_page: i32` in prepared direct INSERT,
  UPDATE, and DELETE metadata, add a cached `PageNumber root`, decode it once at
  prepare time, and use the cached root during direct execution instead of
  calling `page_number_from_schema_root(...)` for every prepared direct DML
  execution.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture --test-threads=1`
    passed with 28 tests.
  - `cargo test -p fsqlite-core direct_simple_update -- --nocapture --test-threads=1`
    passed with 5 tests.
  - `cargo test -p fsqlite-core direct_simple_delete -- --nocapture`
    passed with 1 test.
  - `cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed for both clean baseline and candidate binaries.
- Evidence artifacts:
  `tests/artifacts/perf/prepared-root-page-crimsongorge-20260508T035302Z/summary.md`
  and
  `tests/artifacts/perf/root-page-predecode-calmthrush-20260508T0400Z/summary.md`
  with baseline/candidate focused and full-quick JSON reports.
- Result: rejected. CrimsonGorge's focused UPDATE/DELETE gate worsened weighted
  score `1.1015072810860902 -> 1.2277880043578617`, average ratio
  `1.1138399195630244 -> 1.2418121222187701`, p90/p99
  `1.3975804587368041 -> 1.5230680435137203`, and C-faster rows `2 -> 4`.
  CalmThrush's first full quick looked promising, but the reverse-order repeat
  rejected it: weighted score worsened `0.344815755555221 ->
  0.3498281962295187`, average ratio worsened `0.4497896322365449 ->
  0.4546914761817618`, p90 worsened `1.0229615071185465 ->
  1.0513673719630703`, and C-faster rows increased `9 -> 10`.
- Do not retry prepare-time root `PageNumber` caching as a standalone prepared
  direct DML optimization. Reconsider root metadata only if it is part of a
  broader retained-cursor or row/page-builder design whose same-window full
  quick gate wins on repeat.

## 2026-05-08 - MVCC prepared concurrent-commit page-set `SmallVec`

- Target: low-thread concurrent writer gaps in
  `mt-mvcc-bench --rows-per-thread=1000 --threads=1,2,4,8` and
  `comprehensive-bench --quick --filter concurrent`, especially the remaining
  `2 writers x 1000 rows` row.
- Touched during rejected candidate:
  `crates/fsqlite-mvcc/src/begin_concurrent.rs`; source was restored after
  measurement.
- Candidate shape: keep `PreparedConcurrentCommit::write_set_pages` and
  `held_lock_pages` as `SmallVec<[PageNumber; 16]>` instead of converting the
  common small write/lock page sets into heap `Vec`s. This was narrower than the
  previously rejected one-pass `page_states` scan and did not change validation
  or lock-release semantics.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-mvcc --check` passed.
  - `cargo test -p fsqlite-mvcc commit_updates_commit_index -- --nocapture`
    passed.
  - `cargo test -p fsqlite-mvcc test_prepare_captures_held_lock_pages_separately_from_write_set -- --nocapture`
    passed.
  - `cargo build --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench --bin comprehensive-bench`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/concurrent-profile-calmthrush-20260508T033930Z/summary.md`,
  baseline and candidate `mt-mvcc` JSON/Markdown reports, baseline and
  candidate comprehensive concurrent JSON reports, `perf-mt-2t.data`, and
  `perf-mt-2t-report.txt`.
- Result: rejected and reverted. The focused comprehensive concurrent geomean
  improved `0.7988046779013424 -> 0.7666347556689922`, and the 4/8-writer rows
  improved, but the actual remaining 2-writer gap worsened in both gates:
  standalone throughput ratio `0.73x -> 0.70x` with FSQLite time
  `3.27 ms -> 3.33 ms`, and comprehensive ratio
  `1.0990786735212943 -> 1.1265801605368253` with FSQLite time
  `13.259783 ms -> 14.328216 ms`.
- Do not retry standalone prepared-commit page-set `SmallVec` conversion.
  Reconsider only if a same-window profile proves heap conversion dominates
  low-thread commit cost and the 2-writer row improves without sacrificing the
  4/8-writer rows.

## 2026-05-08 - Prepared direct INSERT row-template executor

- Target: remaining prepared direct INSERT row-build overhead after the
  profile-guided direct INSERT passes, especially expression-shape branching in
  the compiled row serializer.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored after the full quick matrix rejected it.
- Candidate shape: build a per-column record template at prepare time and use
  it to emit SQLite record bytes directly for literal, placeholder, numeric
  binary, and concat expressions. Unsupported expression shapes fell back to the
  existing compiled-row serializer. This applied the query-compilation/template
  specialization idea without unsafe code or runtime JIT.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-row-template-check-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
    completed successfully with 28 tests passing. RCH artifact retrieval hung
    after the remote command had completed, so the local RCH process group was
    terminated.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-row-template-candidate-target CARGO_BUILD_JOBS=12 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed in a clean scratch tree with only the candidate patch applied.
- Evidence artifacts:
  `tests/artifacts/perf/row-template-crimsongorge-20260508T032620Z/summary.md`,
  `candidate-connection.diff`, same-window `baseline-insert.json`,
  `candidate-insert.json`, `candidate-full-quick.json`, and stdout/stderr logs
  under the same directory.
- Result: rejected and reverted. Focused INSERT improved enough to look
  tempting: weighted score `0.8030801931161379 -> 0.7915138891132704`,
  average ratio `0.8290893302098494 -> 0.7957192322438452`, geomean
  `0.8009900880092378 -> 0.7729268013969751`, p99
  `1.2884507148302573 -> 1.1320620924604214`, and C-faster rows `7 -> 5`.
  The full quick gate rejected it: weighted score worsened
  `0.34593878641661835 -> 0.35679620885171676`, average ratio worsened
  `0.4542606463918878 -> 0.4850687497684193`, geomean worsened
  `0.2674752493298549 -> 0.2795497259901094`, p90 worsened
  `0.9811588214938469 -> 1.0870772854107467`, p99 worsened
  `1.4015153360781543 -> 2.091131458001714`, and C-faster rows increased
  `8 -> 11`.
- Do not retry the row-template executor as a standalone direct INSERT
  optimization. Reconsider only as part of a larger row/page builder design that
  protects the large-row full quick rows and wins the full quick weighted score
  in the same A/B window.

## 2026-05-08 - Prepared direct INSERT no-FK guard cache

- Target: remaining no-FK prepared direct INSERT fixed costs, especially
  100-row INSERT rows and the 100-row UPDATE/DELETE setup phase that
  prepopulates through the same direct INSERT path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored after the full quick matrix rejected it.
- Candidate shape: cache `has_outbound_foreign_keys` in
  `PreparedDirectSimpleInsert` at prepare time and consult
  `PRAGMA foreign_keys` only when that bit is true. The proof obligation was
  that prepared statement schema validation already protects the table FK
  layout, while FK-enabled child tables still re-check the pragma dynamically.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fk-direct-insert-candidate-target CARGO_BUILD_JOBS=16 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fk-direct-insert-candidate-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_prepare_insert_with_foreign_keys_uses_direct_dispatch_and_checks_fk -- --nocapture`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/fk-direct-insert-crimsongorge-20260508T030239Z/summary.md`,
  `baseline-insert.json`, `candidate-insert.json`, `baseline-update.json`,
  `candidate-update.json`, `candidate-full.json`, and stdout/stderr logs under
  the same directory.
- Result: rejected and reverted. Focused UPDATE/DELETE improved geomean
  `1.1388583327143484 -> 1.036956189091621`, but INSERT weighted score
  worsened `0.784207453637674 -> 0.7884368705666973` and INSERT p99 worsened
  `1.1516829824326136 -> 1.2355933953670204`. The full quick gate rejected the
  candidate: primary weighted score worsened
  `0.34593878641661835 -> 0.34861836969535076`, average ratio worsened
  `0.4542606463918878 -> 0.4601152352147432`, geomean worsened
  `0.2674752493298549 -> 0.2697448380388971`, p90 worsened
  `0.9811588214938469 -> 1.0592658202932783`, and C-faster rows increased
  `8 -> 10`.
- Do not retry prepare-time no-FK guard caching as a standalone direct INSERT
  optimization. Reconsider FK/no-FK metadata only if it is folded into a broader
  prepared row-template or page-builder change that wins the full quick
  weighted score in the same A/B window.

## 2026-05-08 - File-backed direct INSERT preserialized-record widening

- Target: the remaining low-thread file-backed concurrent writer gap,
  especially `mt-mvcc-bench --rows-per-thread=1000 --threads=2` and the
  comprehensive `2 writers x 1000 rows` concurrent row.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was restored after the comprehensive concurrent gate rejected it.
- Candidate shape: allow the existing prepared-direct INSERT borrowed-parameter
  record serializer on file-backed explicit transactions when the MemDatabase
  row mirror is already unloaded/stale (`in_transaction && !track_memdb_delta`),
  leaving page-run buffering and exact MemDB-delta policies unchanged. The goal
  was to skip cloned `SqliteValue` row construction for write-only concurrent
  transactions without changing storage semantics.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core` passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-file-preserialize-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
    passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-file-preserialize-perf-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin mt-mvcc-bench --profile release-perf`
    passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-file-preserialize-perf-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/file-preserialize-concurrent-crimsongorge-20260508T023131Z/summary.md`,
  `baseline-mt-2t-30.json`, `candidate-mt-2t-30.json`,
  `baseline-concurrent.json`, and `candidate-concurrent.json`.
- Result: rejected and reverted. The standalone 30-iteration `mt-mvcc-bench`
  row improved slightly (FSQLite `4.04 ms -> 3.95 ms`, time ratio
  `1.72x -> 1.65x`), but the comprehensive concurrent filter rejected it:
  `2 writers x 1000 rows` ratio worsened `1.058592 -> 1.117501`, `4 writers`
  ratio worsened `0.974043 -> 1.030069`, and the `8 writers` FSQLite median
  regressed `35.757045 ms -> 44.320340 ms`.
- Do not retry file-backed preserialized-record widening as a standalone
  direct-INSERT optimization. Revisit only with a design that also protects the
  8-writer concurrent row in the same-window comprehensive concurrent filter,
  not just the standalone 2-writer mt harness.

## 2026-05-08 - Concurrent worker PRAGMA fairness probe

- Target: `comprehensive-bench --quick --filter concurrent`, after the current
  concurrent-filter baseline still showed 2-writer and 4-writer ratios near or
  above C SQLite even though the 8-writer row was substantially faster.
- Touched during rejected candidate:
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`; source was manually
  restored after the same-window full quick matrix rejected the candidate.
- Candidate shape: add a concurrent-worker helper that disables time-travel
  snapshot capture, enables `fsqlite.concurrent_mode`, and applies the
  `busy_timeout` on each worker connection in one path. The intent was to remove
  per-worker control-flow variance and make worker setup match the desired
  concurrent mode before the hot loop starts.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-e2e --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-bench-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/concurrent-worker-pragmas-crimsongorge-20260508T0155Z/`
  contains the focused concurrent candidate, same-window full candidate,
  restored-source same-window full baseline, and stdout/stderr logs.
- Result: rejected and reverted. The focused concurrent-only run looked
  promising, but the same-window full quick gate rejected it: primary score
  `0.34107371391864744 -> 0.3630189530149729`, average ratio
  `0.4390016064403414 -> 0.4747587072842754`, geomean
  `0.2634635702889211 -> 0.27568183747093883`, p90
  `0.9699926676890772 -> 1.0625893284142927`, p99
  `1.3974866213697312 -> 1.4108089057410917`, and C-faster rows
  `8 -> 11`. The concurrent rows also worsened in the same full run, including
  8 writers `0.36940401507009746 -> 0.4996207413967345`.
- Do not retry worker PRAGMA ordering or helper consolidation as a standalone
  concurrent benchmark optimization. Reconsider only if a same-window profile
  proves worker setup dominates and the full quick matrix, not just
  `--filter concurrent`, improves.

## 2026-05-08 - Direct prepared INSERT header-size fast path

- Target: `comprehensive-bench --quick --filter insert`, after the direct
  prepared INSERT serializer still had its own
  `prepared_direct_insert_record_header_size()` fixed-point loop even though the
  record-layer one-byte header fast path had already landed.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` in scratch worktree
  `/data/tmp/frankensqlite-direct-header-fastpath-tanbear-20260508T0045Z`; the
  shared checkout was not edited because `connection.rs` was dirty and
  reservation-sensitive.
- Candidate shape: in
  `Connection::prepared_direct_insert_record_header_size(content_size)`, return
  `content_size + 1` immediately for `content_size <= 126`. The proof was that
  the existing loop starts at that value; it is at most 127, so the header-size
  varint is one byte and the loop returns the same value on the first iteration.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-header-fastpath-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-header-fastpath-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/direct-insert-header-fastpath-tanbear-20260508T0045Z/summary.md`,
  `baseline-insert*.json`, `candidate-insert*.json`,
  `direct-header-fastpath.diff`, and `stdout/`.
- Result: rejected. Three alternating focused INSERT pairs showed p99 and some
  non-weighted aggregates improving, but the primary weighted INSERT score was
  worse in all three pairs: `0.803945 -> 0.805853`,
  `0.800119 -> 0.803174`, and `0.763483 -> 0.803002`.
- Do not retry the direct-serializer one-byte header-size shortcut as a
  standalone micro-optimization. Reconsider only if it is folded into a broader
  prepared row-template/page-run writer that wins the focused INSERT score and
  full matrix together.

## 2026-05-08 - Prepared direct INSERT fixed cell array staging

- Target: `comprehensive-bench --quick --filter insert` and `--filter update`,
  after direct INSERT profiling still showed
  `Connection::try_serialize_prepared_direct_simple_insert_record` in the
  row-builder path and UPDATE/DELETE setup remained one of the larger remaining
  gaps.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` in scratch worktree
  `/data/tmp/frankensqlite-direct-cell-array-tanbear-20260508T0025Z`; the shared
  checkout was not edited because `connection.rs` was dirty and
  reservation-sensitive.
- Candidate shape: replace
  `SmallVec<[PreparedDirectInsertRecordCell; 16]>` staging in
  `try_serialize_prepared_direct_simple_insert_record` with a fixed
  `[PreparedDirectInsertRecordCell; 16]` plus `cell_count`, falling back to
  generic dispatch for prepared direct INSERTs with more than 16 columns.
  Intended equivalence for up to 16 columns was identical cell order, serial
  types, header/body sizing, and rowid extraction, with less staging overhead.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-cell-array-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-cell-array-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/record-size-profile-tanbear-20260508T0020Z/direct-cell-array-ab/summary.md`,
  `baseline-insert*.json`, `candidate-insert*.json`,
  `baseline-update*.json`, `candidate-update*.json`,
  `candidate-direct-cell-array.diff`, and `stdout/`.
- Result: rejected. Three same-window focused pairs did not show a reliable
  primary-score win. INSERT non-weighted average, geomean, p90, and p99 mostly
  improved, but weighted score was worse in runs 1 and 3 and only better in run
  2. UPDATE/DELETE was flat to slightly worse versus C SQLite, with C-faster
  rows increasing in the repeat gate.
- Do not retry fixed-array row-cell staging as a standalone `SmallVec` removal.
  Reconsider only inside a broader prepared row-template/direct-DML setup design
  that also preserves the direct fast path for more than 16-column INSERTs and
  proves focused INSERT plus UPDATE/DELETE improvement in the same A/B window.

## 2026-05-08 - Drop retained direct-compiled INSERT AST row values

- Target: prepared direct INSERT setup/prepare overhead after current profiles
  still showed small and medium INSERT rows behind C SQLite, while the direct
  compiled lane never reads `PreparedDirectSimpleInsert::row_values` during
  execution.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after the focused INSERT matrix rejected it.
- Candidate shape: in `prepared_direct_simple_insert_plan`, keep
  `row_values: Vec<Expr>` only for `ReusableTableProgram` direct INSERT plans and
  store an empty vector for `DirectCompiled` plans. Lazy table-program fallback
  still recompiles from the original statement, so the candidate only removed
  unused carried AST state from direct-compiled prepared metadata.
- Correctness proof before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-bench-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_prepared_insert_precomputes_direct_simple_insert_plan -- --nocapture`
  passed.
- Evidence artifacts:
  - Current baseline:
    `tests/artifacts/perf/concurrent-writers-current-crimsongorge-20260508T0110Z/insert-profile-current.json`.
  - Candidate:
    `tests/artifacts/perf/direct-insert-drop-row-values-crimsongorge-20260508T0135Z/insert-candidate.json`
    plus matching stdout/stderr logs.
- Result: rejected and reverted. The target rows were mixed, but the INSERT
  section worsened overall: average ratio `0.8136 -> 0.9000`, geomean
  `0.7874 -> 0.8278`, C-faster rows `7 -> 8`, and only `13/25` absolute
  FrankenSQLite medians improved. Large direct INSERT rows regressed badly,
  including `large_10col` single transaction 10K rows
  `8.702911 ms -> 21.235385 ms` and record-size `large_10col` 10K rows
  `9.057424 ms -> 19.119013 ms`.
- Do not retry dropping or sparsifying retained direct-compiled INSERT AST row
  values as a standalone prepare-state cleanup. Revisit prepared INSERT metadata
  size only if a same-window profile proves prepare metadata allocation, not
  execution/page-write work, dominates and the focused INSERT section improves.

## 2026-05-08 - CellSlotCache full-entry pre-evict

- Target: `comprehensive-bench --quick --filter update`, after the current
  mixed write profile showed `RawVec<CellSlotCacheEntry>::grow_one` at 0.65%
  self time and the remaining full quick matrix still had slow small
  UPDATE/DELETE rows.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`; the
  source was manually restored after the focused benchmark rejected the change.
- Candidate shape: in `CellSlotCache::insert_slow`, pop the LRU tail before
  inserting a new MRU entry when the 64-entry cache is already full. The
  intended equivalence was `insert new MRU then truncate tail` == `pop tail then
  insert new MRU`, while avoiding a transient `Vec` growth from 64 to 128 large
  `CellSlotCacheEntry` values.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-btree --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cursor-candidate-baseline-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-btree cell_slot_cache_evicts_tail_before_full_new_entry_insert -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cell-slot-full-evict-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/cell-slot-full-evict-crimsongorge-20260508T0005Z/summary.md`,
  `update-baseline.json`, `update-candidate.json`, and `stdout/`.
- Result: rejected. Focused UPDATE/DELETE average/geomean worsened
  `0.9809793061876931 / 0.9659518881677094` to
  `1.172453260226592 / 1.1619765793212873`. The target small DELETE row ratio
  improved slightly (`1.385192596298149 -> 1.3550513198997631`), but absolute
  FSQLite time worsened (`0.116298 ms -> 0.118422 ms`), and the larger rows
  regressed materially, including `10000 rows / update 1000 rows`
  `3.450183 ms -> 4.230475 ms` and `10000 rows / delete 500 rows`
  `3.184295 ms -> 4.007728 ms`.
- Do not retry full-cache pre-eviction as a standalone `CellSlotCache`
  micro-optimization. Reconsider only if a future profile proves the 64-to-128
  growth itself dominates and the replacement changes the cache structure more
  fundamentally, with a full matrix gate.

## 2026-05-07 - Prepared direct INSERT append-hint active bit

- Target: `comprehensive-bench --quick --filter insert`, especially page-run
  direct INSERT shapes where the connection crosses generic append-hint
  clear/take sites while the retained table-local hint is usually empty.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  the source was manually restored after the correctness gate failed, before
  any benchmark run.
- Candidate shape: add a `Cell<bool>` beside
  `prepared_direct_insert_append_hint` so clear/take sites can skip the
  `RefCell` borrow when no append hint is parked.
- Correctness proof before measurement: rejected. The focused gate
  `cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
  failed in
  `test_prepared_direct_simple_insert_autocommit_retains_memory_append_hint`
  before benchmarking. Re-running the same focused test after restoring the
  candidate source showed the failure was already present in the current tree
  because the test still expected retained page bytes after the newer B-tree
  staged-page mutation path deliberately clears duplicate page data.
- Result: not a keep. No benchmark was run because the candidate did not clear
  the correctness gate and the code shape adds state that can get out of sync
  with the owned append hint during take/store control flow.
- Do not retry an out-of-band active bit around
  `prepared_direct_insert_append_hint` as a standalone micro-optimization.
  Reconsider only if the hint is refactored into a single owned state machine
  where the presence bit and value cannot diverge.

## 2026-05-07 - Direct UPDATE/DELETE autocommit probe gate hoist

- Target: `comprehensive-bench --quick --filter update`, especially the
  remaining small explicit-transaction direct UPDATE/DELETE rows where fixed
  per-call ceremony is visible after the direct DML fast path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  the source was manually restored immediately after the focused benchmark
  rejected the candidate.
- Candidate shape: compute the `fsqlite.statement_reuse` tracing gate once in
  `execute_precompiled_prepared_update_or_delete`, and skip the autocommit-only
  direct-DML probe when `in_txn_confirmed` is already true. The intent was to
  remove redundant route checks before the explicit-transaction direct lane.
- Correctness proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-gate-hoist-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`
    passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-gate-hoist-bench-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/full-quick-current-crimsongorge-20260507T2242Z/update-baseline-rerun-report.json`,
  `update-dml-gate-hoist-report.json`,
  `stdout/update-baseline-rerun.err`, `stdout/update-dml-gate-hoist.err`,
  `stdout/build-dml-gate-hoist.out`, and `stdout/build-dml-gate-hoist.err`.
  Post-run status also showed an unowned dirty
  `crates/fsqlite-btree/src/cursor.rs` SmallVec candidate in the shared
  checkout, so treat this as a no-keep signal for the route-hoist shape rather
  than a clean standalone A/B proof.
- Result: rejected. Same-window focused update/delete geomean ratio worsened
  from `1.0406236970466178` to `1.2087296154254785`, average ratio worsened
  from `1.0661450609718544` to `1.2287248777917592`, and the high-signal
  small rows regressed (`100 rows / update 10 rows` `0.128681 ms ->
  0.131146 ms`, `100 rows / delete 5 rows` `0.115717 ms -> 0.119514 ms`).
  Larger rows also moved the wrong way, including `10000 rows / update 1000
  rows` `3.751505 ms -> 3.80204 ms` and `10000 rows / delete 500 rows`
  `3.604911 ms -> 3.529159 ms` absolute FSQLite but worse ratios because
  C SQLite moved more in the same window.
- Do not retry this route-check hoist or statement-reuse tracing gate caching
  as a standalone direct UPDATE/DELETE optimization. Reconsider only inside a
  broader retained direct-DML execution design that removes cursor/root-descent
  and route ceremony together, and keep it only if the focused section and full
  quick matrix both improve in the same A/B window.

## 2026-05-07 - Prepared direct INSERT lazy param-one text cache

- Target: `comprehensive-bench --quick --filter insert`, especially tiny and
  medium direct prepared INSERT rows after profiling still showed
  `Connection::try_serialize_prepared_direct_simple_insert_record` in the
  INSERT hot path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`
  in scratch worktree
  `/data/tmp/frankensqlite-param-cache-clean-candidate-tanbear-20260507T2225Z`.
  The shared checkout candidate was manually reverted; no source change was
  kept.
- Candidate shape: add a prepare-time boolean that detects whether a direct
  INSERT expression tree contains a concat chain using `?1`, then format the
  cached `?1` integer text only for those prepared statements instead of
  eagerly formatting it for every integer first parameter.
- Correctness proof before rejection: the shared candidate passed
  `cargo test -p fsqlite-core test_prepared_insert_ -- --nocapture`, and
  `cargo fmt -p fsqlite-core --check` passed after the candidate was reverted.
- Evidence artifacts:
  `tests/artifacts/perf/direct-insert-param-cache-tanbear-20260507T2225Z/baseline-insert.json`,
  `clean-candidate-insert.json`, and `stdout/`.
- Result: rejected and not applied. The clean paired run improved the aggregate
  INSERT average/geomean/p90 ratios from
  `0.8049063998434616 / 0.7782382910450526 / 1.1104506479272773` to
  `0.7944056357714977 / 0.7669999933973539 / 1.100055809067332`, but worsened
  p99 from `1.1792240414399875` to `1.2243380377350952`. The target-ish
  `medium_6col 1000 rows` case regressed badly, with FrankenSQLite median
  `0.491230 -> 0.735477` and ratio `0.836522 -> 1.224338`; `tiny_1col 10000`
  also regressed, with FrankenSQLite median `1.258667 -> 1.486874`.
- Do not retry lazy param-one text caching as a standalone micro-optimization.
  Reconsider only inside a broader prepared row-template or bulk builder that
  removes row-template construction and page-run/page costs together while
  proving INSERT p99 neutrality in the same benchmark window.

## 2026-05-07 - Prepared param-one integer/float binary INSERT specialization

- Target: `comprehensive-bench --quick --filter insert`, after the clean current
  INSERT profile still attributed much of the small/medium row gap to direct
  prepared INSERT row-building and expression/value ceremony.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs` in
  scratch worktree
  `/data/tmp/frankensqlite-paramone-intop-crimsongorge-20260507T2112Z`. The
  shared checkout was not edited because `connection.rs` was peer-reserved for
  a separate DML investigation.
- Candidate shape: add `PreparedDirectSimpleInsertExpr` variants for simple
  `?1 <op> literal` templates, specializing integer `+`, `-`, `*`, `%` and
  float `+`, `-`, `*`, `/` so direct INSERT row construction can avoid recursive
  expression walking and temporary generic value evaluation for benchmark
  row-template columns.
- Correctness/build proof before measurement: the scratch candidate passed
  `cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`;
  both baseline and candidate release-perf `comprehensive-bench` binaries built
  locally with isolated `CARGO_TARGET_DIR`s.
- Evidence artifacts:
  `tests/artifacts/perf/paramone-intop-crimsongorge-20260507T2112Z/summary.md`,
  `baseline-insert.json`, `candidate-insert.json`,
  `baseline-insert-repeat.json`, `candidate-insert-repeat.json`,
  `candidate-paramone-intop.diff`, `repeat-row-compare.tsv`, and `stdout/`.
- Result: rejected and not applied. The first paired run moved the focused
  INSERT primary score `0.8089982083854728 -> 0.7984313729504148` and reduced
  C-faster rows `7 -> 6`, but p99 worsened
  `1.2738419349924275 -> 1.405180687400819`. The repeat again moved the
  primary score slightly `0.7832648833059592 -> 0.7755839688181437`, but
  average/geomean worsened `0.7872773218359899 / 0.7596132132504774` to
  `0.8593629832333529 / 0.8243556684656996`, p99 worsened
  `1.1944813874447535 -> 1.7589035182387223`, and C-faster rows worsened
  `4 -> 6`.
- Do not retry single-expression `?1 op literal` direct INSERT specialization
  as a standalone micro-optimization. Reconsider only as part of a broader
  prepared row-template VM that precomputes the whole direct INSERT column
  program and proves same-window INSERT geomean and p99 wins.

## 2026-05-07 - Direct INSERT page-run admission floor 16 -> 128

- Target: `comprehensive-bench --quick --filter insert`, after the current
  INSERT profile still showed small `single_txn` / transaction-strategy rows
  lagging C SQLite and the existing B-epsilon-style page-run buffer admitted
  almost every no-overflow direct INSERT record at `16` bytes.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`
  in scratch worktree
  `/data/tmp/frankensqlite-pagerun-admission-crimsongorge-20260507T2145Z`.
  The shared checkout was not edited because `connection.rs` and
  `cursor.rs` were exclusively reserved by another agent.
- Candidate shape: raise
  `PREPARED_DIRECT_INSERT_PAGE_RUN_MIN_RECORD_BYTES` from `16` to `128`, while
  leaving `PREPARED_DIRECT_INSERT_PAGE_RUN_ARENA_MAX_RECORD_BYTES` at `384`.
  This tested whether the current admission policy over-buffered small records
  whose batch-buffer overhead outweighed avoided B-tree work.
- Correctness/build proof before measurement: `cargo fmt -p fsqlite-core
  --check` passed in the scratch worktree, and the release-perf
  `comprehensive-bench` binary built successfully. RCH could not normalize the
  `/data/tmp` scratch path and failed open to a local build.
- Evidence artifacts:
  `tests/artifacts/perf/page-run-admission-crimsongorge-20260507T2130Z/summary.md`,
  `candidate-insert-threshold128.json`, `candidate-threshold128.diff`, and
  stdout/stderr under the same directory.
- Result: rejected. The focused INSERT primary score worsened from the current
  profile's `0.8190610418616213` to `0.843813033391191`, average ratio
  worsened `0.8473685692287133 -> 1.0182896196063238`, geomean worsened
  `0.8178930649378039 -> 0.9698058208226569`, p90 worsened
  `1.1504490521484194 -> 1.3905761933793013`, p99 worsened
  `1.2309899121331915 -> 1.5402874678696952`, and C SQLite faster rows
  increased `8 -> 11`. The only headline improvement was `write_single`
  geomean `0.8195249868923463 -> 0.7984774849670893`, which was outweighed by
  write-bulk geomean worsening `0.8176707820510124 -> 0.9958567986163313`.
- Do not raise the direct INSERT page-run admission floor to `128` as a
  standalone expected-loss tweak. Revisit admission only with a richer
  per-record or per-run policy that uses row count, estimated leaf occupancy,
  and flush target shape, and require an INSERT-section win before a full quick
  gate.

## 2026-05-07 - SharedTxnPageIo borrowed concurrent-context clean retry

- Target: `UPDATE/DELETEThroughput`, after fresh isolated `perf record`
  samples for direct-simple DML showed repeated page I/O through
  `SharedTxnPageIo::{read_page_data,write_page_internal}` and per-access
  `ConcurrentContext` cloning on top of the newer direct-INSERT-layout
  baseline. This was a clean retry of the earlier
  `SharedTxnPageIo borrowed concurrent context` rejection because the current
  profile again put the same mechanism near the top of the direct-DML kernel.
- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs`; the
  source was manually restored after the focused and full matrix gates rejected
  the change.
- Candidate shape: borrow `SharedTxnPageIo.concurrent` during hot page-reader
  and page-writer operations instead of cloning the `ConcurrentContext` and its
  shared handles for every page read, page write, dirty check, and witness
  record. A second variant extended the same borrow-only shape to page
  allocation/free and write-witness hooks.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-kernel-crimsongorge-20260507T2010Z/summary.md`,
  `baseline-update.json`, `candidate-update.json`, `candidate2-update.json`,
  `candidate-full-quick*.json`, `candidate2-full-quick.json`, isolated
  `perf-update-delete` stdout/stderr, and `perf-*-report.txt`.
- Result: rejected and reverted. The first narrow variant improved the focused
  UPDATE/DELETE section score `1.1138800909357498 -> 1.0819806080388363` and
  improved isolated `10k` mutation from about `987/1384 ns` per UPDATE/DELETE
  row to `902/1322 ns`, but its first full quick run was effectively flat to
  slightly worse than the current keep artifact (`0.3445386401431955 ->
  0.3447992353725705`) and the repeat was worse (`0.3526452109208745`). The
  expanded variant regressed the focused section to `1.1803042031243598` and
  the full quick score to `0.3548895417230123`, so the matrix keep gate failed.
- Do not retry a standalone `ConcurrentContext` borrow-vs-clone cleanup in
  `SharedTxnPageIo`; this now has two rejected baselines. Reconsider only as
  part of a larger direct-DML batching design that amortizes page I/O setup
  across many row mutations and proves a full quick matrix improvement, not
  only an isolated mutation micro-win.

## 2026-05-07 - Lazy fallback page-lock shard allocation clean retry

- Target: `UPDATE/DELETEThroughput`, after a clean retry of a prior MVCC
  page-lock allocation idea.
- Touched during rejected candidate: `crates/fsqlite-mvcc/src/core_types.rs`;
  the candidate was reverted before this ledger entry was added.
- Candidate shape: change `InProcessPageLockTable.shards` to
  `OnceLock<Box<[LockShard; 64]>>`, preserving the inline fast-lock array and
  allocating fallback hash shards only after the first page above
  `FAST_LOCK_ARRAY_SIZE`.
- Evidence artifacts:
  `tests/artifacts/perf/lazy-fallback-lock-shards-clean-tanbear-20260507T1919Z/summary.md`,
  `candidate-update.json`, and `stdout/`.
- Result: rejected and reverted. Correctness target tests printed green
  (`7 passed; 0 failed; 1 ignored`), and release-perf benchmark build passed,
  but the focused update/delete gate worsened from baseline avg/geomean
  `1.0936760649761197` / `1.073350192601591` to candidate avg/geomean
  `1.1602136307342483` / `1.1419623214888621`. The p90 improved
  `1.5749222579455016 -> 1.5029391100702576`, but the 10K update/delete rows
  flipped from faster to slower (`0.9287127409358253 -> 1.0145355153667168`,
  `0.8785038934276055 -> 1.0715147023000497`), so the matrix keep gate failed.
- Do not retry as a standalone MVCC lock-table allocation change. Reconsider
  only if a future profile proves cold fallback-shard construction dominates a
  startup-only workload and the benchmark gate is intentionally scoped away
  from steady-state 10K DML rows.

## 2026-05-07 - SharedTxnPageIo synthetic page-one cleanup negative cache

- Target: `UPDATE/DELETEThroughput`, after an isolated `perf record` sample
  showed `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` in the
  top hot symbols during repeated direct-simple UPDATE.
- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs`; the
  source was manually restored after the focused section rejected the change.
- Candidate shape: add a shared `Cell<bool>` negative cache to `SharedTxnPageIo`
  so concurrent page I/O starts conservative, probes page-one synthetic
  conflict tracking once, then skips repeated cleanup probes until allocator,
  free-page, or page-one tracking paths mark cleanup as possible again.
- Correctness/build proof before measurement:
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo fmt -p fsqlite-vdbe --check`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-vdbe page_one -- --nocapture`
    passed locally after an RCH retrieval wrapper hung post-pass.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`,
  `update-pageone-negative-cache.json`,
  `stdout/update-pageone-negative-cache.out`, and
  `stdout/update-pageone-negative-cache.err`.
- Result: rejected. The focused section gate moved the wrong way: geomean ratio
  `1.1564512197233796 -> 1.2540719758886116`, average ratio
  `1.1677116353247705 -> 1.2694155421623223`, comparable rows `2 -> 0`, and
  C SQLite faster rows `4 -> 6`. Some large FSQLite medians improved
  (`10000 update` `4.281429 ms -> 3.986017 ms`, `10000 delete`
  `3.927927 ms -> 3.683761 ms`), but the small-row rows regressed and the
  section matrix failed.
- Do not retry a per-adapter boolean negative cache for page-one synthetic
  cleanup as a standalone direct UPDATE/DELETE optimization. Reconsider only if
  paired with a lower-level write-surface redesign that removes the per-row
  write-page ceremony without adding a hot branch to every write.

## 2026-05-07 - Same-size UPDATE staged-page overwrite probe

- Target: `UPDATE/DELETEThroughput`, especially fixed-width REAL direct UPDATE
  rows where an isolated `perf record` sample showed self-time in
  `PageData::as_bytes_mut`, `write_page_internal`, and staged write-surface
  maintenance under `table_overwrite_current_payload_same_size_no_overflow`.
- Touched during rejected candidates:
  `crates/fsqlite-btree/src/cursor.rs` and, for the adaptive retry,
  `crates/fsqlite-core/src/connection.rs`. The source was manually restored
  after the focused section rejected both variants.
- Candidate shapes:
  - Unconditional B-tree variant: before cloning and re-submitting the whole
    leaf page for a same-size overwrite, call `try_mutate_staged_page_data` and
    patch an already-staged leaf image in place.
  - Adaptive retry: keep the default overwrite path for the first 64 executions
    of a prepared fixed-width REAL direct UPDATE, then switch repeated loops to
    the staged-page probe via a separate prefer-staged overwrite method.
- Correctness/build proof before measurement:
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo fmt -p fsqlite-btree -p fsqlite-core --check`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-btree table_overwrite_current_payload_same_size_no_overflow -- --nocapture`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core direct_simple_update -- --nocapture`
    passed for the adaptive retry.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`
    passed for both measured variants.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`,
  `update-staged-overwrite.json`, `update-adaptive-staged-overwrite.json`,
  `stdout/update-staged-overwrite.err`,
  `stdout/update-adaptive-staged-overwrite.err`,
  `stdout/perf-update-delete-staged-overwrite-isolated.err`, and
  `stdout/perf-update-delete-adaptive-staged-overwrite-isolated.err`.
- Result: rejected. The unconditional variant improved some large absolute
  FSQLite medians (`10000 rows / update 1000 rows` `4.281429 ms ->
  3.806522 ms`, `10000 rows / delete 500 rows` `3.927927 ms -> 3.470382 ms`)
  but failed the section gate: geomean ratio
  `1.1564512197233796 -> 1.1706410749024634`, average ratio
  `1.1677116353247705 -> 1.1885956679385385`, and small-row ratios worsened.
  The adaptive retry was worse: geomean ratio
  `1.1564512197233796 -> 1.274136716373797`, average ratio
  `1.1677116353247705 -> 1.336137408656441`, and the 100-row DELETE row
  regressed from `0.120676 ms` to `0.188122 ms`.
- Do not retry staged-page same-size overwrite probing as a standalone direct
  UPDATE optimization. Reconsider only if the B-tree/pager API can patch the
  authoritative staged page and cursor stack without triggering `PageData`
  copy-on-write or extra control branches, and require same-window improvement
  in the focused section, not just large-row absolute medians.

## 2026-05-07 - Hard-disable dormant QF consultation in direct UPDATE/DELETE

- Target: `UPDATE/DELETEThroughput`, especially the per-row direct-simple
  UPDATE/DELETE path that still calls `qf_maybe_short_circuit_for_rowid` even
  though build-on-first-consult was disabled by `4ea55010` after a severe
  full-table scan regression.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
  The performance part was manually reverted after measurement. The stale
  quotient-filter tests were separately corrected to assert the current
  disabled build-on-first-consult semantics.
- Candidate shape: add an explicit always-false
  `QUOTIENT_FILTER_CONSULTATION_ENABLED` guard at the top of
  `qf_maybe_short_circuit_for_rowid`, so direct UPDATE/DELETE avoids even the
  dormant `RefCell<HashMap<...>>` lookup when no current production path creates
  quotient-filter entries.
- Correctness/build proof before measurement:
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo fmt -p fsqlite-core --check`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core quotient_filter -- --nocapture`
    passed after updating the stale QF tests.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`,
  `update-qf-dormant-consult.json`, `stdout/update-current.err`, and
  `stdout/update-qf-dormant-consult.err`.
- Result: rejected. The focused section moved the wrong way: geomean ratio
  `1.1564512197233796 -> 1.2592198894797841`, average ratio
  `1.1677116353247705 -> 1.2930116072309275`, p90
  `1.4506585635858391 -> 1.9091211450460472`. The 100-row UPDATE row worsened
  from `0.128421 ms` to `0.166462 ms`, and the 10000-row DELETE row worsened
  from `3.927927 ms` to `4.079655 ms`, despite 1000-row rows moving slightly
  faster in the noisy candidate run.
- Do not retry an unconditional hard-off QF consultation branch as a standalone
  direct UPDATE/DELETE optimization. Reconsider QF only with a complete
  activation redesign where entry creation, absent-key benefit, and present-key
  overhead are all measured in the same real benchmark slice.

## 2026-05-07 - Lazy VDBE fallback compilation for direct UPDATE/DELETE

- Target: `UPDATE/DELETEThroughput`, especially the small prepared direct
  UPDATE/DELETE rows where `prepare_us` is a visible part of total time.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  the source was restored with manual reverse patches after the focused
  benchmark rejected the candidate.
- Candidate shape: for prepared direct-simple UPDATE/DELETE statements, store
  the shared placeholder `VdbeProgram` at prepare time and compile the real
  table UPDATE/DELETE program lazily only if forced fallback, tracing, or a
  direct-path `NotImplemented` bailout reaches the reusable table-program
  dispatcher.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core` passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 rch exec -- cargo test -p fsqlite-core test_prepared_update_delete_precompute_statement_savepoint_skip_hint -- --nocapture`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_prepared_update_delete_forced_fallback_use_instrumented_lane -- --nocapture`
    passed after proving the lazy fallback execution path.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo check -p fsqlite-core --all-targets`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`,
  `update-lazy-ud-program.json`, `stdout/update-current.err`, and
  `stdout/update-lazy-ud-program.err`.
- Result: rejected. The candidate did reduce measured prepare ceremony on the
  smallest UPDATE row (`fs_update_100 prepare_us` about `24.7 -> 13.1`), but
  the section gate moved the wrong way: geomean ratio
  `1.1564512197233796 -> 1.2736550244705815`, average ratio
  `1.1677116353247705 -> 1.2812755890557856`, comparable rows `2 -> 0`, and
  C SQLite faster rows `4 -> 6`. The 100-row UPDATE/DELETE rows measured
  `0.128421 ms` / `0.120676 ms` at baseline versus `0.1179 ms` /
  `0.1177 ms` in the noisy candidate run, but the larger rows regressed enough
  to fail the matrix gate, including `1000 delete 50` `0.409336 ms ->
  0.5102 ms`.
- Do not retry lazy UPDATE/DELETE fallback compilation by itself. The saved
  prepare work is not the dominant section cost once full-row setup, mutation,
  and commit timing are measured together. Reconsider only if paired with a
  fallback-free direct DML plan where the reusable table-program path is no
  longer needed for forced/traced execution.

## 2026-05-07 - Retained direct UPDATE/DELETE cursor shell

- Target: `UPDATE/DELETEThroughput`, especially repeated prepared rowid
  UPDATE/DELETE loops inside one explicit concurrent transaction.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`
  and `crates/fsqlite-vdbe/src/engine.rs`; the source was restored with a
  reverse patch after the matrix rejected the candidate.
- Candidate shape: expose the existing `SharedTxnPageIo` drain/refill pattern
  used by retained VDBE storage cursors, then keep a connection-local
  `BtCursor<SharedTxnPageIo>` for direct-simple UPDATE/DELETE. The cache was
  gated by root page, page size, reserved bytes, schema generation, concurrent
  session id, and `total_changes`, and reused `BtCursor::advance_to` on later
  rowid probes to avoid root descent.
- Correctness/build proof before measurement:
  - `cargo fmt --check` passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 rch exec -- cargo check -p fsqlite-core --all-targets`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 rch exec -- cargo test -p fsqlite-core test_direct_simple_update_delete -- --nocapture`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-next-target CARGO_BUILD_JOBS=16 rch exec -- cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next-crimsongorge-20260507T1710Z/update-current.json`,
  `update-retained-dml.json`, `stdout/update-current.err`,
  `stdout/update-retained-dml.err`, `stdout/rebuild-retained-dml.err`, and
  the earlier `perf-update-delete` isolated outputs in the same artifact
  bundle.
- Result: rejected. The focused update/delete matrix worsened materially:
  summary geomean ratio `1.1564512197233796 -> 1.3768322577729717`, average
  ratio `1.1677116353247705 -> 1.392106249500981`, comparable rows `2 -> 0`,
  and C SQLite faster rows `4 -> 6`. FrankenSQLite row timings worsened on
  every measured row, including `100 update 10` `0.128421 ms -> 0.1447 ms`,
  `1000 update 100` `0.447378 ms -> 0.4926 ms`, and `10000 update 1000`
  `4.281429 ms -> 5.14 ms`.
- Do not retry a connection-local retained direct-DML cursor shell as a
  standalone optimization, even when it uses `advance_to`. A retry is only
  justified if the design also changes the mutation primitive itself, for
  example a same-leaf batch update/delete API that avoids per-row
  drain/refill, payload copy, and delete rebalance ceremony under one cursor
  borrow.

## 2026-05-07 - Direct UPDATE/DELETE microbatch schema-proof carry

- Target: `UPDATE/DELETEThroughput`, especially repeated prepared rowid
  UPDATE/DELETE loops inside one explicit transaction.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  the source was manually restored after the repeat A/B benchmark lost.
- Candidate shape: allow the statement microbatcher to carry the
  schema/function proof for direct-simple UPDATE/DELETE despite the
  conservative `may_observe_change_tracking` flag, gated by the existing
  direct-simple eligibility, statement-savepoint elision, fused-entry auto
  mode, no rollback conflict action, and an active explicit transaction.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed before the candidate was
    formatted.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_stmt_microbatch_coalesces_repeated_direct_update_delete -- --nocapture`
    passed after moving the hook from the INSERT precompiled branch to the
    actual deferred direct-DML fast branch.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core direct_simple_update -- --nocapture`
    passed.
  - `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-patch-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/summary.md`,
  `report-update-delete-samewindow-baseline*.json`,
  `report-update-delete-microbatch-candidate*.json`,
  `stdout-samewindow-baseline*.txt`,
  `stderr-samewindow-baseline*.txt`,
  `stdout-microbatch-candidate*.txt`, and
  `stderr-microbatch-candidate*.txt`. Repeat paired A/B follow-up:
  `tests/artifacts/perf/direct-update-delete-microbatch-purpleotter-20260507T1159Z/summary.md`,
  `baseline-update.json`, `candidate-update.json`,
  `baseline-update-profile.json`, `candidate-update-profile.json`, and
  `baseline-perf-*` / `candidate-perf-*`.
- Result: rejected. Three same-window A/B pairs had average section geomean
  ratio `1.2150058278690452` for baseline versus
  `1.247387599103826` for candidate, and average FSQLite-only geomean
  worsened from `0.6078380658848437 ms` to `0.6189310105900363 ms`. Large
  delete sometimes improved, but medium rows regressed and the ratio-to-C gate
  lost on two of three runs. The PurpleOtter repeat showed the same stop
  condition in a cleaner paired run: no-profile section geomean moved
  `1.2245037883938406 -> 1.1289754225301574`, but the isolated per-row harness
  rejected the candidate on the target update path (`635 -> 694 ns/row` at 100
  rows, `782 -> 827 ns/row` at 1000 rows, `844 -> 907 ns/row` at 10000 rows)
  and delete also failed to improve (`1156 -> 1161`, `1103 -> 1164`,
  `1221 -> 1248 ns/row`).
- Do not retry schema-proof carry for direct UPDATE/DELETE as a standalone
  optimization. The avoided schema proof is not the dominant cost; revisit
  only if it falls out naturally inside a retained direct-DML cursor/run design
  that removes per-row cursor construction/root descent.

## 2026-05-07 - Direct UPDATE/DELETE per-row scratch reset removal

- Target: `UPDATE/DELETEThroughput`, especially direct UPDATE/DELETE per-row
  ceremony in prepared statement loops.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  the source was manually restored after the focused benchmark lost.
- Candidate shape: remove `PreparedDirectInsertScratchResetGuard` from
  `execute_prepared_direct_simple_update` and
  `execute_prepared_direct_simple_delete`, relying on each direct DML path's
  existing scratch clears plus commit-time statement-lookaside reset.
- Correctness/build proof before measurement:
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target cargo fmt -p fsqlite-core --check`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target cargo test -p fsqlite-core direct_simple_update -- --nocapture`
    passed, with the expected temporary dead-code warning because the guard
    became unused during the candidate.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-patch-target cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`
    passed, with the same temporary warning.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/summary.md`,
  `report-update-delete.json`, `report-update-delete-scratchreset-candidate.json`,
  `stderr-scratchreset-candidate.txt`, and
  `stdout-scratchreset-candidate.txt`.
- Result: rejected. The candidate had mixed row noise and worsened the section:
  geomean ratio `1.1514568045449403 -> 1.1827616752954908`, `10000 rows /
  update 1000 rows` `4.282235 ms` / `1.16851603333226 -> 4.374428 ms` /
  `1.1500152479099848`, and `10000 rows / delete 500 rows` `3.942168 ms` /
  `1.1384892008417877 -> 4.068265 ms` / `1.1455357987592527`. The small
  update row improved (`0.132028 ms -> 0.126537 ms`) but not enough to justify
  the broader regression.
- Do not retry removing the direct UPDATE/DELETE scratch reset as a standalone
  ceremony trim. Reconsider only if paired with a larger retained-cursor design
  and revalidated on the full `UPDATE/DELETEThroughput` section.

## 2026-05-07 - Direct UPDATE fixed-width REAL leaf-payload patch

- Target: `UPDATE/DELETEThroughput`, especially the top full-matrix gap
  `100 rows / update 10 rows`, and the isolated direct UPDATE mutation loop.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`; the source was manually removed
  after the focused benchmark lost.
- Candidate shape: add a B-tree primitive that parses the current leaf-table
  record payload in-place, verifies the target column is a fixed-width REAL
  serial type, and patches only the 8 value bytes. Call it from
  `try_execute_prepared_direct_simple_update_fixed_width_real` before the
  existing `payload_into` plus whole-payload overwrite fallback.
- Correctness/build proof before measurement:
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target cargo test -p fsqlite-btree test_table_patch_current_payload_fixed_width_real_updates_only_target_column -- --nocapture`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-review-target cargo test -p fsqlite-core direct_simple_update -- --nocapture`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-patch-target cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/summary.md`,
  `report-update-delete.json`, `report-update-delete-candidate.json`,
  `stderr.txt`, `stderr-candidate.txt`, `stdout.txt`, and
  `stdout-candidate.txt`. Read-only repeat/no-profile follow-up:
  `tests/artifacts/perf/fixed-real-update-purpleotter-20260507T1128Z/summary.md`,
  `report-update.json`, `report-update-noprofile.json`, and
  `perf-10000-update.txt`.
- Result: rejected. The candidate eliminated payload-copy counters on UPDATE
  (`btree_payload_copy_calls=1000` / `btree_payload_copy_bytes=20889` became
  `0 / 0` on `fs_update_10000`) but worsened the focused section:
  geomean ratio `1.1514568045449403 -> 1.2399807521821862`, `100 rows /
  update 10 rows` `0.132028 ms` / `1.5145515239810492 -> 0.134542 ms` /
  `1.5662448632728374`, and `10000 rows / update 1000 rows` `4.282235 ms` /
  `1.16851603333226 -> 4.337518 ms` / `1.173031026575702`. The repeat
  evaluation also rejected it: profiled update-filter geomean only moved
  `1.1514568045449403 -> 1.1494521577224535` (within noise), while the
  no-profile update filter reported geomean `1.1983325033541927` and regressed
  the latest full-matrix large rows from `0.975043` / `0.935698`
  (10K update/delete) to `1.157850` / `1.152931`. Isolated
  `perf-update-delete` still measured FSQLite update at `641 ns`/row for the
  100-row workload and `892 ns`/row for the 10K workload.
- Do not retry a standalone leaf-payload byte patch for fixed-width REAL
  direct UPDATE. The copied payload is too small to justify the extra B-tree
  primitive and record-header parse in isolation. Reconsider only as part of a
  retained direct-DML cursor/run design that also removes per-row cursor
  construction/root descent and can cache the payload offset across repeated
  prepared executions.

## 2026-05-07 - Non-empty direct INSERT page-run via append hint

- Target: `INSERTThroughput - Transaction Strategy Comparison (small_3col)`,
  especially the remaining `10000 rows / batched (1000/txn)` gap versus C
  SQLite.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`
  and `crates/fsqlite-btree/src/cursor.rs`. The connection-level buffering
  source was removed after measurement. The B-tree cursor-stack guard and
  regression test discovered during correctness testing were kept as a separate
  correctness fix.
- Candidate shape: after the first normal right-edge insert established a
  retained append hint, start a pending direct INSERT page-run for subsequent
  explicit rowids that are strictly greater than the hint's last rowid, then
  flush that non-empty run at the normal read/savepoint/commit boundary by
  replaying rows through one hot cursor.
- Correctness/build proof before measurement:
  - `TMPDIR=/data/tmp cargo fmt -p fsqlite-btree -p fsqlite-core --check`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-nonempty-pagerun-btree-target cargo test -p fsqlite-btree test_table_append_after_last_position_repeated_after_existing_rows_crosses_split -- --nocapture`
    passed after adding the cursor-stack guard.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-nonempty-pagerun-target cargo test -p fsqlite-core prepared_direct_insert_page_run -- --nocapture`
    passed before measurement and passed again after the rejected
    `connection.rs` changes were removed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-nonempty-pagerun-perf-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/nonempty-pagerun-purpleotter-20260507T1030Z/summary.md`,
  `candidate-transaction.json`, `candidate-stdout.txt`,
  `candidate-stderr.txt`, `btreeguard-transaction.json`, and
  `btreeguard-full.json`.
- Result: rejected. The candidate reduced target-row cursor setup
  (`cursor_setup_ns` from about `410210` to `14856`) but worsened the target
  row in absolute time and ratio: retained append-hint baseline
  `4.289494 ms` / `1.3290062724722278` became `4.76 ms` /
  `1.4408254018260138`. Profile counters show the work moved to commit-time
  full-cell replay: `commit_us=3524.8`, `btree_cell_assembly_calls=9000`,
  `btree_leaf_full_cell_appends=8943`, and `btree_leaf_payload_appends=0`.
  With the rejected `connection.rs` changes removed, the retained B-tree guard
  did not reproduce this regression: the focused transaction row measured
  `4.251529 ms` / `1.2777334488590038`, and the full quick matrix completed
  with primary weighted score `0.36394897123082987`.
- Do not retry append-hint-started non-empty page-run buffering if the flush
  path replays row-at-a-time full-cell appends at commit. Reconsider only with a
  true non-empty page builder or direct payload-writer flush that preserves the
  payload-append kernel, and require an absolute FSQLite median improvement on
  `10000 rows / batched (1000/txn)` before any full-matrix repeat.

## 2026-05-07 - Depth-2 non-empty right-edge bulk append flush hook

- Target: `INSERTThroughput - Transaction Strategy Comparison (small_3col)`,
  especially the remaining `10000 rows / batched (1000/txn)` gap versus C
  SQLite.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`; the source was manually removed
  after the focused measurement lost on the target row.
- Candidate shape: add a narrow
  `table_bulk_append_depth2_right_edge_sorted_records` primitive for monotonic
  appends after a depth-2 table root, then call it from direct INSERT page-run
  flush paths after the existing empty-root bulk loader and before row-at-a-time
  append replay.
- Correctness/build proof before measurement:
  - `TMPDIR=/data/tmp cargo fmt -p fsqlite-btree -p fsqlite-core` passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-target cargo test -p fsqlite-btree test_table_bulk_append_depth2_right_edge_sorted_records_extends_tree -- --nocapture`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-target cargo test -p fsqlite-core test_prepared_direct_insert_page_run_flushes_before_read -- --nocapture`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-target cargo test -p fsqlite-core test_prepared_direct_simple_insert_executes_inside_explicit_transaction -- --nocapture`
    passed.
  - `TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-perf-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
    passed.
- Evidence artifacts:
  `tests/artifacts/perf/current-gap-refresh-purpleotter-20260507T1000Z/summary.md`,
  `report-transaction.json`, `candidate-transaction.json`, `stdout.txt`,
  `stderr.txt`, `candidate-stdout.txt`, and `candidate-stderr.txt`.
- Result: rejected. The transaction-section aggregate improved
  (`primary weighted score 1.0031958927979898 -> 0.9054630878224692`), but the
  target row moved the wrong way: FSQLite median worsened
  `4.305162 ms -> 4.376543 ms`, and the ratio worsened
  `1.2894486040035198 -> 1.3574438929737422`. The hot-row profile counters
  stayed effectively unchanged (`btree_leaf_payload_appends=8934`,
  `btree_quick_balance_hits=57`, `btree_conservative_reloads=57` in both
  runs), which indicates the new flush hook did not materially affect the row
  it was meant to fix.
- Do not retry this as a btree-only primitive plus flush hook. Reconsider only
  after connection-level buffering can safely form non-empty monotonic INSERT
  runs and a focused proof shows the new primitive is actually invoked and the
  `10000 rows / batched (1000/txn)` FSQLite median improves before a full quick
  matrix repeat.

## 2026-05-07 - Broad depth-2 right-edge page-builder admission

- Target: the same non-empty right-edge transaction row,
  `INSERTThroughput - Transaction Strategy Comparison (small_3col)` /
  `10000 rows / batched (1000/txn)`, plus the broader INSERT section.
- Touched during candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs` in CrimsonGorge's dirty shared
  worktree. TanBear measured the candidate read-only and did not edit, stage,
  or revert those files.
- Candidate shape: retry the depth-2 primitive with the missing
  connection-level run formation, so pending direct INSERT page-runs are
  materialized as whole right-edge leaf pages and parent divider cells instead
  of replaying rows through `table_append_after_last_position`.
- Legacy SQLite comparison: `sqlite3BtreeInsert()` uses `BTREE_APPEND` to bias
  the cursor seek, and `balance_quick()` handles the append split by moving one
  overflow cell to one new right sibling and inserting one parent divider cell.
  The dirty candidate is more aggressive: it admits a whole buffered run and
  splices multiple new leaves into the depth-2 parent. That is the right retry
  direction for the target row, but the matrix shows the broad admission is too
  coarse as-is.
- Evidence artifacts:
  `tests/artifacts/perf/right-edge-depth2-tanbear-20260507T1417Z/summary.md`,
  `clean-fullquick.json`, `dirty-fullquick.json`,
  `dirty-transaction-repeat.json`, `dirty-update-profile.json`, and the focused
  repeat in
  `tests/artifacts/perf/right-edge-depth2-insert-repeat-tanbear-20260507T1431Z/summary.md`,
  `clean-insert.json`, and `dirty-insert.json`.
- Result: not safe to land as a broad standalone admission. The full quick
  same-window run improved the primary weighted score `0.370335 -> 0.368076`
  and the target row moved sharply in the right direction (`4.475514 ms` /
  `1.354616x` to `2.502868 ms` / `0.779247x`). The INSERT-only repeat
  confirmed the target win (`4.336625 ms -> 2.507627 ms`) but worsened the
  INSERT-section primary score `0.902771 -> 0.921972` and increased C-faster
  INSERT rows `8 -> 9`. Regressions included `1000 rows / autocommit`
  `0.728394 ms -> 1.012487 ms`, `tiny_1col` 100 rows
  `0.069290 ms -> 0.081132 ms`, `large_10col` 100 rows
  `0.169909 ms -> 0.184546 ms`, record-size `large_10col` 10K
  `10.815963 ms -> 11.579393 ms`, and `small_3col` 10K single transaction
  `2.895064 ms -> 3.001402 ms`.
- Do not retry or land the broad depth-2 page-builder admission based on the
  target row alone. Revisit only by narrowing admission so it fires for the
  proven batched non-empty right-edge row while excluding the small-row,
  autocommit, and large-record shapes that regressed, then publish a fresh
  source-owned full quick matrix showing the weighted score and INSERT section
  both clear the keep gate.

## 2026-05-07 - Retained autocommit direct INSERT page-run widening

- Target: remaining `:memory:` autocommit INSERT transaction-strategy gap where
  profiles showed repeated cursor setup and full-cell assembly in prepared
  direct INSERT.
- Touched during abandoned candidate: `crates/fsqlite-core/src/connection.rs`
  in scratch worktree
  `/data/tmp/frankensqlite-retained-pagerun-crimsongorge-20260507T0904Z`;
  source was not applied to the shared checkout.
- Candidate shape: widen direct page-run buffering eligibility to retained
  autocommit prepared inserts and add a flush bridge so pending runs would be
  applied before retained autocommit reads.
- Correctness result: abandoned before measurement. The focused test
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-retained-pagerun-crimsongorge-target cargo test -p fsqlite-core test_prepared_direct_insert_retained_autocommit_page_run_flushes_before_read -- --nocapture`
  failed because no pending page run materialized.
- Root cause: retained autocommit prepared INSERT already uses the append-hint
  path after the first row, while page-run buffering requires
  `prepared_append_hint.is_none()`. Trying to make page-run cross retained
  autocommit was the wrong seam for this workload.
- Do not retry retained-autocommit page-run widening as a standalone
  optimization. Reconsider only if profiles show append hints cannot apply for
  a specific prepared INSERT shape and a correctness proof shows the page-run
  flush boundary is read-your-own-write safe.

## 2026-05-07 - Right-edge byte-slice payload append from current cursor

- Target: transaction-strategy INSERT rows and write-single/write-bulk matrix
  rows where the current profile showed autocommit staying on
  `btree_leaf_full_cell_appends` while explicit/batched paths could use the
  cheaper table-leaf payload append kernel.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`; the
  source was manually removed before this ledger entry was committed.
- Candidate shape: add a byte-slice variant of the existing right-edge
  `try_append_table_leaf_payload_in_place_no_overflow` path and route
  `table_append_after_last_position` through it before falling back to the
  normal full-cell insert path. This was distinct from the previously rejected
  writer-callback/direct-record append candidates.
- Correctness/build proof before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-table-append-payload-target cargo test -p fsqlite-btree test_table_append_after_last_position_uses_payload_append_when_leaf_has_room -- --nocapture`
  passed, and the candidate release-perf build succeeded at
  `/data/tmp/frankensqlite-table-append-payload-candidate-target/release-perf/comprehensive-bench`.
- Evidence artifacts:
  `tests/artifacts/perf/table-append-payload-purpleotter-20260507T0900Z/summary.md`,
  `baseline-insert.json`, `candidate-insert.json`, `candidate-full.json`,
  `baseline-full-repeat.json`, `candidate-full-repeat.stderr`, and
  `candidate-full-repeat2.stderr`.
- Result: rejected/abandoned. The insert section improved primary score
  `1.187148 -> 1.166217` and geomean `0.945056 -> 0.911234`; the first full
  quick run also looked promising with primary score `0.380185 -> 0.358372`.
  The mandatory full-matrix repeat failed twice before producing JSON, both
  times panicking in the 8-writer concurrent benchmark with
  `fsqlite COMMIT ... failed: database is busy (snapshot conflict on pages: ) (retry_count=64)`.
- Do not retry this as a local right-edge byte-slice append patch unless the
  design includes an explicit concurrent-writer correctness proof, a focused
  multi-writer stress gate, and a same-window full quick repeat that completes
  successfully. The first-run score is not a keep signal when repeat matrix
  completion fails.

## 2026-05-07 - Private memory page-cache fallback shard fanout

- Target: remaining small private `:memory:` setup cost in UPDATE/DELETE and
  transaction-strategy rows where profiles showed pager/page-cache construction
  and fresh database fixed costs.
- Touched during rejected candidate: `crates/fsqlite-pager/src/page_cache.rs`
  and `crates/fsqlite-pager/src/pager.rs`; the reservation holder reverted the
  source before commit.
- Candidate shape: route private `/:memory:` pager opens to a
  single-connection page-cache constructor that uses `MIN_PAGE_CACHE_SHARDS` for
  the sharded fallback tier before the flat-array fast path takes over. Normal
  file-backed/shared pager construction kept the default page-cache fanout.
- Correctness/build proof reported by PurpleOtter:
  - `cargo fmt -p fsqlite-pager --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-page-cache-shards-local-target cargo test -p fsqlite-pager single_connection_initial_page_hint_keeps_fallback_shards_small -- --nocapture`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-page-cache-shards-local-target cargo check -p fsqlite-pager -p fsqlite-core`
    passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-page-cache-shards-local-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`
    passed.
- Conflicting evidence:
  - PurpleOtter's focused UPDATE/DELETE repeats in
    `tests/artifacts/perf/memory-page-cache-shards-purpleotter-20260507T080110Z/`
    worsened section score twice (`1.176988 -> 1.207445` and
    `1.071504 -> 1.272281`), with the 10K update row regressing.
  - CrimsonGorge's read-only full quick matrix in
    `tests/artifacts/perf/private-page-cache-shards-crimsongorge-20260507T0755Z/`
    improved the primary weighted score (`0.3746319462 -> 0.3716428852`),
    C-faster rows (`16 -> 14`), p90 (`1.2032368062 -> 1.1215995512`), and
    write-single geomean (`1.2265527627 -> 1.1470306548`), while geomean
    slightly worsened (`0.2763330429 -> 0.2775776435`).
- Result: rejected/abandoned by the reservation holder because focused
  UPDATE/DELETE failed and the source was reverted. Treat the current evidence
  as inconclusive rather than a clean universal negative: the focused section
  and full matrix disagreed.
- Do not retry this as a standalone private page-cache shard reduction unless a
  same-window run includes both repeated UPDATE/DELETE focused gates and a full
  quick matrix, with both moving in the right direction. A future retry should
  also include the candidate artifact in Git so the exact source diff and
  benchmark basis are reproducible.

## 2026-05-07 - Lazy fallback page-lock shard allocation

- Target: remaining fresh `:memory:` connection/open fixed cost after profiles
  showed allocator time under `SharedMvccState::new` and after the accepted
  lower `LOCK_TABLE_SHARDS` fanout still left eager fallback lock-shard
  construction in `InProcessPageLockTable::new`.
- Touched during rejected candidate: `crates/fsqlite-mvcc/src/core_types.rs`;
  source was reverted before commit.
- Candidate shape: change `InProcessPageLockTable.shards` from eager
  `Box<[LockShard; LOCK_TABLE_SHARDS]>` to `OnceLock<Box<[LockShard; ...]>>`,
  allocate fallback shards only on first page number above
  `FAST_LOCK_ARRAY_SIZE`, keep fast-array page locks allocation-free, and make
  count/holder/release paths avoid allocating when the fallback table is absent.
  Rebuild paths used `OnceLock::take()` to rotate an empty table only when
  maintenance requested it.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-mvcc --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazy-fallback-shards-target cargo test -p fsqlite-mvcc in_process_lock_table -- --nocapture`
    passed 8 matching lock-table tests, with 1 ignored microbench.
  - Candidate release-perf build passed with
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazy-fallback-shards-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`.
- Evidence:
  `tests/artifacts/perf/lazy-fallback-lock-shards-crimsongorge-20260507T0820Z/`
  records the attempted measurement, but the benchmark basis is confounded:
  while the candidate was being built, the private page-cache shard diff used
  as the intended baseline was reverted in the shared tree and a separate dirty
  `crates/fsqlite-core/src/connection.rs` candidate appeared. The JSON is
  retained for audit only, not as standalone performance proof.
- Result: abandoned and reverted, with correctness proven but no valid
  same-source A/B. The recorded focused transaction/full quick numbers compare
  different dirty-tree states and must not be used as the reason to keep or
  reject the idea.
- Do not retry standalone lazy fallback page-lock shard allocation from this
  artifact. Reconsider only in a clean worktree or after the active
  `connection.rs` candidate lands/reverts, with baseline and candidate source
  states pinned and a same-window full quick matrix.

## 2026-05-07 - Private memory retained-autocommit flush threshold 256 -> 1024

- Target: remaining `INSERTThroughput - Transaction Strategy Comparison
  (small_3col)` gap where 10K `autocommit` and `batched (1000/txn)` rows still
  trail C SQLite while single-transaction rows are already faster.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted before commit.
- Candidate shape: keep mixed read/write retained-autocommit flushing at `16`
  statements and file-backed pure writes at `256`, but raise the pure-write
  private `:memory:` threshold to `1024` so long insert loops pay fewer retained
  flush boundaries.
- Correctness/build proof before measurement:
  - `cargo fmt -p fsqlite-core --check` passed.
  - `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-autocommit-threshold-target cargo test -p fsqlite-core retained_autocommit_adaptive_flush -- --nocapture`
    passed the retained-autocommit adaptive-threshold tests, including a
    temporary memory-specific assertion for the candidate.
  - Candidate release-perf build passed with
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-autocommit-threshold-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Same-window evidence:
  `tests/artifacts/perf/private-autocommit-threshold-crimsongorge-20260507T0750Z/`
  compared `comprehensive-bench --quick --filter transaction` against the
  current baseline binary at
  `/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target`.
- Result: rejected. The apparent 10K autocommit ratio improvement
  (`1.345x -> 1.263x`) came from a slower C SQLite median in the candidate run;
  absolute FrankenSQLite time worsened (`11.044968 ms -> 11.279396 ms`).
  Other target rows also regressed, including 100-row autocommit
  (`0.147657 ms -> 0.153147 ms`), 100-row batched
  (`0.092022 ms -> 0.117490 ms`), and 1K autocommit
  (`1.129896 ms -> 1.189378 ms`). The only target-family win was a tiny 10K
  batched improvement (`4.501872 ms -> 4.481204 ms`), not enough to justify the
  broader regressions.
- Do not retry a standalone retained-autocommit threshold increase. Reconsider
  only if a phase profile proves retained flush boundaries dominate the actual
  autocommit row and a same-window transaction-section/full-quick A/B improves
  absolute FrankenSQLite medians, not just ratios.

## 2026-05-07 - Lazy page-lock waiter-shard allocation

- Target: remaining fixed allocation/open cost in small `:memory:`
  UPDATE/DELETE rows after profiles still showed allocator and
  `SharedMvccState::new` / `InProcessPageLockTable` setup cost.
- Touched during rejected candidate: `crates/fsqlite-mvcc/src/core_types.rs`;
  source was reverted before commit.
- Candidate shape: change `InProcessPageLockTable` waiter queues from eagerly
  allocated `Box<[WaiterShard; LOCK_TABLE_SHARDS]>` to a `OnceLock` directory,
  allocating a waiter shard only when `register_waiter` actually parks a thread
  on a page lock. `waiter_count == 0` fast paths and targeted wake semantics
  were preserved.
- Correctness proof before measurement:
  - `cargo fmt -p fsqlite-mvcc --check` passed.
  - `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazy-waiter-target cargo test -p fsqlite-mvcc in_process_lock_table -- --nocapture`
    printed `7 passed; 0 failed; 1 ignored`; the RCH wrapper later hung in
    artifact retrieval after the green test result and was terminated locally.
- Focused evidence:
  `tests/artifacts/perf/lazy-waiter-shards-crimsongorge-20260507T0715Z/`
  compared the candidate against the current-HEAD baseline binary at
  `/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target`. 100-row
  UPDATE moved only `1538ns -> 1520ns` per updated row, while 100-row DELETE
  regressed `2417ns -> 2453ns` per deleted row.
- Matrix evidence:
  - Baseline:
    `tests/artifacts/perf/lock-table-shards-64-purpleotter-20260507T064123Z/report-full.json`.
  - Candidate:
    `tests/artifacts/perf/lazy-waiter-shards-crimsongorge-20260507T0715Z/report-full.json`.
- Result: rejected. The full quick matrix moved the primary score in the wrong
  direction (`0.3705736243 -> 0.3818466951`), worsened geomean
  (`0.2773103795 -> 0.2876165270`), worsened C-faster rows (`14 -> 17`), and
  worsened `write_single` geomean (`1.1928924299 -> 1.2099582428`). A small
  `write_bulk` geomean improvement did not offset the matrix regression.
- Do not retry standalone lazy waiter-shard allocation. Reconsider only if a
  future profile shows waiter-shard construction as retained self-time after a
  broader lock-table/open-state redesign, and require a same-window full quick
  matrix improvement.

## 2026-05-07 - Lazy conflict ring-buffer allocation

- Target: repeated `SharedMvccState::new` cost in small `:memory:` write and
  update/delete rows, after profiles showed connection-open allocation cost.
- Touched during rejected candidate:
  `crates/fsqlite-observability/src/lib.rs`; the candidate was never applied to
  the main checkout.
- Candidate shape: change `ConflictRingBuffer` construction from
  `Vec::with_capacity(capacity)` to `Vec::new()`, preserving the configured
  ring capacity while deferring the event storage allocation until the first
  conflict event.
- Correctness proof before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-observer-candidate-test cargo test -p fsqlite-observability ring -- --nocapture`
  passed all 11 matching observability/ring tests.
- Focused evidence: `perf-update-delete 100 5000 delete fsqlite standard`
  stayed within noise at `2304ns` per deleted row in
  `tests/artifacts/perf/lazy-conflict-ring-crimsongorge-20260507T0552Z/perf-delete-100-fsqlite-standard.txt`.
- Matrix evidence:
  - Baseline:
    `tests/artifacts/perf/lazy-commit-index-chunks-crimsongorge-20260507T0506Z/report-full.json`.
  - Candidate:
    `tests/artifacts/perf/lazy-conflict-ring-crimsongorge-20260507T0552Z/report-full.json`.
- Result: rejected. The full quick matrix moved the primary score in the wrong
  direction (`0.3781791993813428 -> 0.39476918213037665`), worsened p99
  (`1.5325130971275822 -> 2.2593567303940096`), and worsened C-faster rows
  (`15 -> 18`).
- Do not retry this standalone lazy ring-buffer allocation. Reconsider only if
  conflict observability is redesigned as a fully lazy/optional subsystem and
  the full quick matrix improves.

## 2026-05-07 - Stack-backed empty page-1 bootstrap

- Target: small `:memory:` write/update/delete rows where profiles showed
  repeated empty-database open cost, including `vec![0; page_size]` page-1
  bootstrap allocation under `Connection::open_with_page_size_and_env`.
- Touched during rejected candidate: `crates/fsqlite-pager/src/pager.rs`; the
  candidate was reverted before commit.
- Candidate shape: use a stack `[u8; 4096]` scratch page for empty database
  bootstrap when `page_size <= PageSize::DEFAULT`, falling back to the existing
  heap `Vec` for larger pages.
- Focused evidence: `perf-update-delete 100 5000 delete fsqlite standard`
  improved from the accepted lazy-commit-index profile run's `2310ns` per
  deleted row to `2253ns` in
  `tests/artifacts/perf/stack-bootstrap-page1-crimsongorge-20260507T0543Z/perf-delete-100-fsqlite-standard.txt`.
- Matrix evidence:
  - Baseline:
    `tests/artifacts/perf/lazy-commit-index-chunks-crimsongorge-20260507T0506Z/report-full.json`.
  - Candidate:
    `tests/artifacts/perf/stack-bootstrap-page1-crimsongorge-20260507T0543Z/report-full.json`.
- Result: rejected. The full quick matrix moved the primary score in the wrong
  direction (`0.3781791993813428 -> 0.38512660315341524`), worsened
  C-faster rows (`15 -> 19`), and regressed `write_single` geomean
  (`1.1537259815867404 -> 1.2390301694080836`) despite the focused DELETE
  probe looking better.
- Do not retry stack-only page-1 bootstrap. Reconsider only as part of a larger
  empty-open redesign that also reduces the surrounding VFS write/sync and
  `SharedMvccState::new` costs, and keep it only if the full quick matrix
  improves.

## 2026-05-07 - CommitIndex lazy-chunk get-first subvariant

- Target: `CommitIndex` lazy fast-array chunks after the broader lazy-chunk
  candidate improved `SharedMvccState::new` fixed costs in the full quick
  matrix.
- Touched during rejected subvariant: `crates/fsqlite-mvcc/src/core_types.rs`;
  the subvariant was reverted before commit.
- Candidate shape: change `CommitIndex::fast_slot` to call `OnceLock::get()`
  before falling back to `get_or_init()`, trying to trim steady-state
  `get_or_init` overhead after a chunk had already been allocated.
- Correctness proof before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazycommit-target cargo test -p fsqlite-mvcc commit_index -- --nocapture`
  passed.
- Evidence artifacts:
  - Better lazy-chunk baseline:
    `tests/artifacts/perf/lazy-commit-index-chunks-crimsongorge-20260507T0506Z/report-full.json`.
  - Rejected get-first subvariant:
    `tests/artifacts/perf/lazy-commit-index-chunks-fastget-crimsongorge-20260507T0516Z/report-full.json`.
- Result: rejected. The full quick matrix moved the primary score in the wrong
  direction (`0.3781791993813428 -> 0.39347654243834473`) and worsened
  C-faster rows (`15 -> 19`). The `write_single` geomean regressed
  `1.1537259815867404 -> 1.2854579276323546`, with 100-row update/delete back
  above `1.59x`.
- Do not retry this standalone `get()`-first `OnceLock` subvariant. Reconsider
  only as part of a larger measured commit-index batch writer that reduces
  per-page chunk lookup work without hurting the full quick matrix.

## 2026-05-06 - Single-freeblock compact table-leaf DELETE

- Target: `UPDATE/DELETEThroughput`, especially small direct DELETE rows where
  profiles showed table-leaf DELETE paying page-copy/defrag costs after every
  point rowid deletion.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`; the
  source was reverted immediately after measurement.
- Candidate shape: in `remove_table_cell_from_leaf_deferred`, let an otherwise
  compact table leaf avoid full defragmentation for one DELETE by either
  advancing `cell_content_offset` when deleting the low physical boundary cell
  or writing a single SQLite-format freeblock for the deleted cell. Pages that
  already had a freeblock or fragmented bytes still used the existing eager
  defrag path, avoiding the historical multi-freeblock-chain corruption mode
  fixed by `5eed5a0a`.
- Correctness/build proof before measurement:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-single-freeblock-target cargo test -p fsqlite-btree cursor_delete -- --nocapture`
  passed, and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-single-freeblock-release cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete --bin comprehensive-bench`
  passed.
- Evidence artifacts:
  `tests/artifacts/perf/delete-single-freeblock-purpleotter-20260506T2351Z/delete-100-standard.json`,
  `tests/artifacts/perf/delete-single-freeblock-purpleotter-20260506T2351Z/update-baseline.json`,
  and
  `tests/artifacts/perf/delete-single-freeblock-purpleotter-20260506T2351Z/update-candidate.json`.
- Result: rejected and reverted. The focused `perf-update-delete 100 20000
  delete fsqlite standard` hyperfine probe showed only a noisy win
  (`4.593s +/- 0.449s` baseline vs `4.386s +/- 0.220s` candidate,
  `1.05x +/- 0.11`). The actual Section 6 same-window gate rejected it:
  average ratio worsened `1.84x -> 2.01x`, and absolute FrankenSQLite medians
  worsened on every row (`100` update `235.4us -> 330.9us`, `100` delete
  `223.0us -> 337.7us`, `1K` update `624.5us -> 724.1us`, `1K` delete
  `627.8us -> 697.8us`, `10K` update `4.47ms -> 5.44ms`, `10K` delete
  `4.13ms -> 5.46ms`).
- Do not retry a standalone single-freeblock shortcut for compact table-leaf
  DELETE. Revisit table-leaf freeblocks only with a correctness proof that
  exercises SQLite `btreeComputeFreeSpace()`-compatible layouts and a
  same-window Section 6 matrix improvement in absolute FrankenSQLite medians,
  not just a focused harness win.

## 2026-05-07 - Direct DML lookaside growth guard elision

- Target: remaining small prepared direct INSERT/UPDATE/DELETE gaps where
  per-row `StatementLookasideGrowthGuard` construction/drop still performed
  dormant hot-path profiling checks with profiling disabled.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs` in
  detached scratch worktree
  `/data/tmp/frankensqlite-crimsongorge-lookaside-20260506T2355`; the main
  worktree was not changed.
- Candidate shape: add
  `StatementLookasideGrowthGuard::new_when_profiled(conn, profile_enabled) ->
  Option<Self>` and use it in prepared direct INSERT/UPDATE/DELETE so the normal
  non-profiling path avoids the guard's drop-time retained-byte sampling check.
- Correctness/build proof before measurement:
  `cargo fmt -p fsqlite-core --check` passed in the scratch worktree, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lookaside-target cargo check -p fsqlite-core --lib`
  passed.
- Evidence artifacts:
  - Baseline clean HEAD full quick:
    `tests/artifacts/perf/current-head-after-revert-crimsongorge-20260506T2329Z/report-full.json`.
  - Candidate scratch INSERT:
    `tests/artifacts/perf/lookaside-guard-scratch-crimsongorge-20260506T2357Z/report-insert.json`.
  - Candidate scratch UPDATE/DELETE:
    `tests/artifacts/perf/lookaside-guard-scratch-crimsongorge-20260506T2357Z/report-update.json`.
- Result: rejected. INSERT was mixed and too small to justify touching the hot
  path: 100-row tiny improved only `0.180638 ms -> 0.175158 ms`, while 100-row
  small regressed `0.188052 ms -> 0.191478 ms` and 100-row transaction-strategy
  single-txn regressed `0.111549 ms -> 0.112912 ms`. The target UPDATE/DELETE
  rows moved sharply the wrong way: 100-row update regressed
  `0.151935 ms -> 0.248596 ms`, and 100-row delete regressed
  `0.145312 ms -> 0.241422 ms` with high candidate CVs.
- Do not retry standalone `StatementLookasideGrowthGuard` elision or
  `Option<Guard>` plumbing in direct DML. Revisit lookaside profiling overhead
  only if a profile with profiling disabled shows the guard or
  `note_statement_lookaside_alloc_growth` as retained self-time, and require a
  same-window INSERT plus UPDATE/DELETE section win before applying it to the
  main worktree.

## 2026-05-07 - sqlite_master rightmost rowid allocation

- Target: fixed CREATE TABLE/setup overhead on the remaining small INSERT and
  UPDATE/DELETE rows after profiling showed the DML mutation itself was already
  fast and setup/insert dominated the gap.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted immediately after the focused INSERT gate rejected it.
- Candidate shape: replace the defensive full sqlite_master cursor scan in
  `insert_sqlite_master_row_with_sql` and writable-schema raw sqlite_master
  insertion with a rightmost-row lookup, floor `next_master_rowid` against that
  max rowid, then append through `table_append_after_last_position`. This kept
  the stale-counter and schema-hole invariant while avoiding O(n) sqlite_master
  scans on ordinary DDL.
- Correctness proof before measurement:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-master-rowid-target cargo test -p fsqlite-core next_master_rowid -- --nocapture`
  passed the schema reload and schema-hole rowid tests. The test command itself
  finished successfully; a later `rch` artifact retrieval rsync hung and was
  terminated after the green test result.
- Evidence artifacts:
  - Baseline current INSERT profile:
    `tests/artifacts/perf/current-insert-profile-crimsongorge-20260507T0232Z/report-insert.json`.
  - Candidate INSERT section:
    `tests/artifacts/perf/sqlite-master-rowid-rightmost-crimsongorge-20260507T0310Z/report-insert.json`.
- Result: rejected and reverted. The focused INSERT primary score regressed
  `1.31098 -> 1.42125`, geomean regressed `1.09237 -> 1.23222`,
  C-faster rows increased `14 -> 17`, write-bulk regressed
  `1.05507 -> 1.19917`, and write-single regressed `1.40941 -> 1.50406`.
  Absolute FrankenSQLite medians also regressed in too many rows, including
  `small_3col` 100 rows (`0.123301 ms -> 0.248335 ms`), `medium_6col` 1000
  rows (`0.561542 ms -> 0.756547 ms`), and `large_10col` 10K rows
  (`11.917479 ms -> 14.639497 ms`), despite a few fixed-cost rows improving.
- Do not retry sqlite_master full-scan replacement, rightmost-row allocation,
  or append-position reuse as a standalone setup optimization. Revisit only if
  a CREATE TABLE-specific profile shows sqlite_master rowid allocation as
  retained self-time and a same-window full quick run improves both the primary
  score and the write-bulk/write-single category scores.

## 2026-05-07 - Pager committed-cache direct insertion

- Target: INSERT commit overhead after profiling on clean `edccd638` showed
  `SimpleTransaction<MemoryVfs>::commit`, `flush_write_set_to_db_file_batch`,
  and `drain_committed_cache_pages` under the remaining write-bulk rows.
- Touched during rejected candidate: `crates/fsqlite-pager/src/pager.rs`;
  source was reverted after measurement.
- Candidate shape: replace
  `drain_committed_cache_pages() -> Vec<(PageNumber, PageBuf)>` with direct
  insertion of drained staged pages into `self.cache` while checking
  `inner.commit_seq`, avoiding the temporary committed-page vector allocation.
- Correctness/build proof before measurement: `cargo fmt -p fsqlite-pager --check`
  passed after formatting, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-pager-cache-clean-target cargo check -p fsqlite-pager --lib`
  passed in the clean `edccd638` worktree.
- Evidence artifacts:
  - Clean baseline full quick:
    `tests/artifacts/perf/clean-edccd-full-purpleotter-20260507T013900Z/report-full.json`.
  - Candidate insert section:
    `tests/artifacts/perf/pager-cache-direct-purpleotter-20260507T0201Z/report-insert.json`.
- Result: rejected and reverted. Insert-only average ratio worsened to `1.36x`;
  C-faster rows increased to `16/25`. Large-row regressions dominated the
  result: `large_10col` 10K single transaction worsened from `12.011853 ms`
  in the clean full baseline to `21.604153 ms`, and record-size `large_10col`
  worsened from `11.486259 ms` to `21.567394 ms`. Smaller rows were mixed and
  did not justify touching commit/cache semantics.
- Do not retry direct cache insertion or temporary-Vec removal in this pager
  path as a standalone perf change. Revisit only if a profile shows
  `drain_committed_cache_pages` itself, not downstream cache insertion/page
  ownership, as dominant retained self-time and a same-window INSERT plus full
  quick A/B improves absolute FrankenSQLite medians.

## 2026-05-07 - Empty-root bulk-loader duplicate leaf grouping reuse

- Target: remaining multi-page explicit INSERT page-run flush cost after the
  current profile showed large/medium 10K rows still paying commit/page-build
  work even though per-row B-tree insertion was already batched.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`;
  source was reverted after the same-window INSERT section rejected it.
- Candidate shape: in `BtCursor::table_bulk_load_empty_root_sorted_records`,
  reuse the already-computed root leaf groups as the ordinary leaf groups when
  `root_header_offset == 0`. Ordinary table roots do not carry SQLite's page-1
  database header prefix, so the root-fit planning pass and the child-leaf
  planning pass produce the same groups. Page-1 roots kept the existing
  recomputation path.
- Correctness/build proof before measurement: `cargo fmt -p fsqlite-btree --check`
  passed,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-bulkgroup-target cargo test -p fsqlite-btree table_bulk_load_empty_root_sorted_records -- --nocapture`
  passed the focused reachable-tree test,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-bulkgroup-target cargo check -p fsqlite-btree --lib`
  passed, and the release-perf `comprehensive-bench` binary built.
- Evidence artifacts:
  - Candidate INSERT section:
    `tests/artifacts/perf/bulkgroup-reuse-crimsongorge-20260507T0339Z/report-insert.json`.
  - Same-window restored-baseline INSERT section:
    `tests/artifacts/perf/bulkgroup-reuse-baseline-crimsongorge-20260507T0347Z/report-insert.json`.
- Result: rejected and reverted. The focused INSERT primary score worsened
  `1.31251 -> 1.35832`, geomean worsened `1.06435 -> 1.10294`,
  average ratio worsened `1.12949 -> 1.17948`, p90 worsened
  `1.52685 -> 1.62401`, and p99 worsened `2.29999 -> 2.50693`.
  Write-bulk geomean worsened `1.02270 -> 1.06005`, write-single geomean
  worsened `1.42633 -> 1.47535`, and absolute FrankenSQLite medians worsened
  in `17/25` insert rows. The target large rows regressed materially:
  record-size `large_10col` 10K worsened by `1.153270 ms`, and single-transaction
  `large_10col` 10K worsened by `1.409400 ms`, despite isolated improvements
  such as `medium_6col` 1000 rows improving by `0.185448 ms`.
- Do not retry leaf-group planning reuse or duplicate grouping removal inside
  the existing empty-root bulk loader as a standalone optimization. Revisit
  only with a true fused planner/page builder that avoids both the grouping pass
  and per-page payload rewrites, and require a same-window INSERT section win
  before running the full quick matrix.

## 2026-05-07 - Direct INSERT record-cell layout reuse

- Target: prepared direct INSERT row-build/record serialization after perf
  profiles showed `Connection::try_serialize_prepared_direct_simple_insert_record`
  and direct-record layout work in the INSERT hot path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`
  in scratch worktrees only; the shared checkout was not edited because the file
  was reserved.
- Candidate shape: replace the serializer's separate
  `SmallVec<[(serial_type, payload_len); 16]>` layout pass with a
  `PreparedDirectInsertRecordCell { value, serial_type, payload_len }` collected
  during `try_serialize_prepared_direct_simple_insert_record`, so serialization
  reuses the layout computed after affinity application.
- Correctness/build proof before measurement: `cargo fmt -p fsqlite-core --check`
  passed in scratch, and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-layoutreuse-check-target cargo check -p fsqlite-core --lib`
  passed. The broad `test_prepared_direct_simple_insert` group had one
  pre-existing clean-main failure in
  `test_prepared_direct_simple_insert_executes_inside_explicit_transaction`, so
  it was not candidate-specific.
- Evidence artifacts:
  - Scratch worktree:
    `/data/projects/frankensqlite-layoutreuse-purpleotter-rch-20260507T0340Z`.
  - Baseline INSERT section:
    `/data/tmp/frankensqlite-layoutreuse-purpleotter-20260507T0340Z/baseline-insert.json`.
  - Candidate INSERT section:
    `/data/tmp/frankensqlite-layoutreuse-purpleotter-20260507T0340Z/candidate-insert.json`.
- Result: rejected. The candidate improved insert avg/geomean
  (`1.26249 -> 1.23337`, `1.17377 -> 1.13823`) and some large rows, but failed
  the project keep gate: primary weighted score worsened `1.35791 -> 1.37669`,
  write-single geomean worsened `1.43874 -> 1.48463`, p90 worsened
  `1.63282 -> 1.75325`, and p99 worsened slightly `2.56035 -> 2.56613`.
  Notable absolute FSQLite regressions included `small_3col` 1000
  single transaction (`0.331651 ms -> 0.443140 ms`) and 10K batched
  (`4.341832 ms -> 4.587422 ms`).
- Do not retry direct-record layout reuse as a standalone connection-layer
  change. Revisit only if the same idea is part of a fused bulk page/record
  builder and a same-window full quick run improves primary score plus
  write-bulk/write-single category scores.

## 2026-05-07 - FunctionRegistry copy-on-write map clone

- Target: fixed per-connection startup overhead after 100-row INSERT and
  UPDATE/DELETE profiles showed the remaining gap was dominated by connection,
  schema, setup, and prepare costs rather than mutation time.
- Touched during rejected candidate: `crates/fsqlite-func/src/lib.rs`; source
  was reverted immediately after the full quick matrix rejected it.
- Candidate shape: change `FunctionRegistry`'s scalar, aggregate, and window
  maps from owned `HashMap`s to `Arc<HashMap>` and use `Arc::make_mut` for
  registration. This made `FunctionRegistry::clone_from_arc` cheap for maps
  that a connection does not mutate, while preserving the public registry API.
- Correctness/build proof before measurement: `cargo fmt -p fsqlite-func --check`
  passed,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-func-cow-target cargo check -p fsqlite-func --lib`
  passed, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-func-cow-target cargo test -p fsqlite-func`
  passed with `404` tests, `0` failures, and `13` ignored perf tests.
- Evidence artifacts:
  - Baseline full quick:
    `tests/artifacts/perf/current-clean-full-crimsongorge-20260507T0252Z/report-full.json`.
  - Candidate full quick:
    `tests/artifacts/perf/function-registry-cow-crimsongorge-20260507T0400Z/report-full.json`.
- Result: rejected and reverted. The full quick primary weighted score
  regressed `0.4256727367 -> 0.4275996880`; average ratio regressed
  `0.6289344096 -> 0.6745824780`; p99 regressed
  `2.5838276864 -> 4.5616830120`; write-bulk geomean regressed
  `1.1826104374 -> 1.2775170175`; and write-single geomean regressed
  `1.2880277067 -> 1.4351847605`. The worst fixed-cost target row,
  `tiny_1col` 100-row single transaction, worsened to `4.56x` over C SQLite,
  and large 10K rows also regressed badly.
- Do not retry internal `FunctionRegistry` COW map cloning as a standalone
  startup optimization. Revisit function-registry cloning only if a direct
  connection-open profile shows registry map cloning as retained self-time and
  a same-window full quick run improves both primary score and write category
  geomeans.

## 2026-05-07 - SharedTxnPageIo borrowed concurrent context

- Target: hot page I/O in INSERT and UPDATE/DELETE after profiles showed
  `TransactionKind::get_page` and shared transaction page-state checks on the
  remaining write-side gap rows.
- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs`;
  source was reverted after the full quick matrix rejected it.
- Candidate shape: replace repeated `SharedTxnPageIo::concurrent_context()`
  clones in read/write witness probes, dirty checks, staged mutation, and page
  read/write paths with scoped immutable borrows of the `RefCell<Option<_>>`,
  keeping the same concurrent-writer defaults and page-level MVCC behavior.
- Correctness/build proof before measurement: `cargo fmt -p fsqlite-vdbe --check`
  passed, `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-vdbe-context-target cargo check -p fsqlite-vdbe --lib`
  passed, `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-vdbe-context-target cargo clippy -p fsqlite-vdbe --lib -- -D warnings`
  passed, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-vdbe-context-target cargo test -p fsqlite-vdbe shared_txn_page_io -- --nocapture --test-threads=1`
  passed.
- Evidence artifacts:
  - Baseline with the retained rowid-bucket SUM fast path:
    `tests/artifacts/perf/rowid-bucket-sum-main-full-crimsongorge-20260507T0035Z/report-full.json`.
  - Candidate focused UPDATE/DELETE:
    `tests/artifacts/perf/vdbe-context-borrow-crimsongorge-20260507T0120Z/update-delete-report.json`.
  - Candidate full quick:
    `tests/artifacts/perf/vdbe-context-borrow-crimsongorge-20260507T0120Z/report-full.json`.
- Result: rejected. The full quick weighted score regressed
  `0.419154 -> 0.442052`, C-faster rows increased `21 -> 23`, p90 regressed
  `1.4466 -> 1.5119`, p99 regressed `2.9264 -> 4.7195`, write-bulk geomean
  regressed `1.2056 -> 1.2306`, and write-single geomean regressed
  `1.3334 -> 1.4426`. The 100-row update row became much worse
  (`0.211296 ms -> 0.524753 ms` FSQLite median, ratio
  `1.617x -> 4.719x`), despite a few larger UPDATE/DELETE rows moving within
  noise.
- Do not retry standalone `SharedTxnPageIo` concurrent-context clone removal.
  Revisit only if a profile shows `concurrent_context()` cloning as retained
  self-time and the replacement can avoid broad `RefCell` borrow lifetimes while
  winning the same-window full quick weighted score.

## 2026-05-07 - Exact connection PRAGMA execute fast path

- Target: small INSERT and transaction-strategy gaps where every benchmark
  connection pays parser/planner/VDBE setup for repeated exact connection PRAGMA
  setters before the measured work begins.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after the focused INSERT matrix rejected it.
- Candidate shape: add a narrow `Connection::execute` pre-parse fast path for
  exact `PRAGMA name = value` assignments covering `page_size`, `journal_mode`,
  `synchronous`, `cache_size`, and the time-travel capture flag. Anything with
  quoted values, extra statements, comments, invalid values, or query semantics
  fell back to the normal parser path.
- Correctness/build proof before measurement:
  `cargo fmt -p fsqlite-core` passed after formatting,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-pragma-fastpath-target cargo check -p fsqlite-core --lib`
  passed,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-pragma-fastpath-target cargo test -p fsqlite-core test_exact_connection_pragma_execute_fast_path_updates_state -- --nocapture`
  passed, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-pragma-fastpath-target cargo clippy -p fsqlite-core --lib -- -D warnings`
  passed.
- Evidence artifacts:
  - Baseline with the retained rowid-bucket SUM fast path:
    `tests/artifacts/perf/rowid-bucket-sum-main-full-crimsongorge-20260507T0035Z/report-full.json`.
  - Candidate focused INSERT:
    `tests/artifacts/perf/exact-pragma-fastpath-crimsongorge-20260507T0120Z/report-insert.json`.
- Result: rejected. The focused INSERT report still had C SQLite faster in
  `16/25` rows, with weighted score `1.2329`, geomean `1.1586`, p90 `1.4344`,
  p99 `2.9919`, write-bulk geomean `1.1449`, and write-single geomean
  `1.2637`. Against the current full-matrix baseline rows, ratios improved in
  only `11/25` INSERT rows and worsened in `14/25`; absolute FrankenSQLite
  medians worsened in `22/25` rows. The target small rows were mixed: 100-row
  medium improved by ratio (`2.9264x -> 1.3870x`) and large improved
  (`2.1649x -> 1.0196x`), but tiny worsened (`2.4459x -> 2.9919x`) and most
  larger rows moved the wrong way.
- Do not retry engine-level exact PRAGMA setter bypass as a standalone
  benchmark optimization. Revisit PRAGMA setup only if a profile shows measured
  benchmark time, not setup time, is actually dominated by connection PRAGMA
  execution, and require a same-window full quick improvement before keeping it.

## 2026-05-07 - Prepared direct INSERT param-literal arithmetic variant

- Target: prepared direct INSERT row-building cost after
  `FSQLITE_BENCH_PROFILE_INSERT=1` showed `prepared_direct_insert_row_build`
  dominating the remaining large INSERT path: about `1.73 ms` for 10K
  `small_3col`, `2.96 ms` for 10K `medium_6col`, and `5.98 ms` for 10K
  `large_10col`, while B-tree insert time was much smaller.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after the focused INSERT matrix rejected it.
- Candidate shape: add a compiled `PreparedDirectSimpleInsertExpr` variant for
  `?N <arithmetic-op> literal` and `literal <arithmetic-op> ?N`, so benchmark
  expressions such as `?1 * 0.137`, `?1 * 7`, `?1 * 13`, and `?1 % 20` avoid
  recursive boxed `BinaryOp` evaluation inside the direct-record serializer.
- Correctness/build proof before measurement:
  `cargo fmt -p fsqlite-core` passed,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-expr-specialize-target cargo check -p fsqlite-core --lib`
  passed,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-expr-specialize-target cargo test -p fsqlite-core test_prepared_insert_precomputes_direct_simple_insert_plan -- --nocapture`
  passed, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-expr-specialize-target cargo clippy -p fsqlite-core --lib -- -D warnings`
  passed.
- Evidence artifacts:
  - Profile lead:
    `tests/artifacts/perf/insert-profile-current-crimsongorge-20260507T0135Z/report.json`.
  - Baseline with the retained rowid-bucket SUM fast path:
    `tests/artifacts/perf/rowid-bucket-sum-main-full-crimsongorge-20260507T0035Z/report-full.json`.
  - Candidate focused INSERT:
    `tests/artifacts/perf/param-literal-expr-specialize-crimsongorge-20260507T0140Z/report-insert.json`.
- Result: rejected. The focused INSERT report had weighted score `1.2873`,
  geomean `1.1560`, p90 `1.9553`, p99 `3.3952`, write-bulk geomean `1.1326`,
  and write-single geomean `1.3434`. Against the retained baseline rows,
  ratios improved in `17/25` INSERT rows, but absolute FrankenSQLite medians
  worsened in `21/25`, including target rows such as 1K `small_3col`
  (`0.425377 ms -> 0.762548 ms`), 1K `medium_6col`
  (`0.510446 ms -> 0.814745 ms`), and 10K `large_10col`
  (`11.618710 ms -> 12.921337 ms`). The headline worst rows also remained bad:
  tiny 100 regressed by ratio (`2.4459x -> 3.3952x`) and 100-row batched
  transaction strategy regressed (`1.6883x -> 3.1575x`).
- Do not retry a standalone param-literal arithmetic enum variant in prepared
  direct INSERT. Revisit row construction only with a broader template/fused
  serializer that proves lower absolute FrankenSQLite medians on the focused
  INSERT section and then wins the full quick weighted score.

## 2026-05-07 - Exact transaction-control execute fast path

- Target: remaining 100-row INSERT and transaction-strategy gaps where the
  measured work includes exact `BEGIN`/`COMMIT` calls through
  `Connection::execute`.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after the focused INSERT matrix rejected it.
- Candidate shape: add a narrow pre-parse recognizer for exact transaction
  commands (`BEGIN`, `BEGIN TRANSACTION`, `BEGIN CONCURRENT`, `COMMIT`/`END`,
  and exact `ROLLBACK`) and dispatch them straight to the existing transaction
  helpers. Comment-bearing SQL, multi-statements, savepoint forms, and
  `BEGIN IMMEDIATE`/`EXCLUSIVE`/`DEFERRED` fell back to the parser. Plain
  `BEGIN` still called `execute_begin` with `mode: None`, preserving
  `concurrent_mode_default` auto-promotion.
- Correctness/build proof before measurement:
  `cargo fmt -p fsqlite-core --check` passed,
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-txn-fastpath-target cargo test -p fsqlite-core test_exact_transaction_execute_skips_sql_parse -- --nocapture`
  passed,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-txn-fastpath-target cargo check -p fsqlite-core --lib`
  passed, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-txn-fastpath-target cargo clippy -p fsqlite-core --lib -- -D warnings`
  passed.
- Evidence artifacts:
  - Baseline with the retained rowid-bucket SUM fast path:
    `tests/artifacts/perf/rowid-bucket-sum-main-full-crimsongorge-20260507T0035Z/report-full.json`.
  - Candidate focused INSERT:
    `tests/artifacts/perf/exact-txn-fastpath-crimsongorge-20260507T0215Z/report-insert.json`.
- Result: rejected. The focused INSERT report had weighted score `1.6036`,
  geomean `1.1419`, p90 `1.7562`, p99 `2.8941`, write-bulk geomean
  `1.0704`, and write-single geomean `1.8349`. Against the retained baseline
  INSERT rows, ratios improved in `12/25` rows and worsened in `13/25`, but
  absolute FrankenSQLite medians worsened in `23/25` rows. Notable regressions
  included 100-row autocommit (`0.186940 ms -> 0.493755 ms`), 1K
  `medium_6col` (`0.510446 ms -> 1.014921 ms`), 10K `large_10col`
  (`11.618710 ms -> 14.872844 ms`), and 10K batched transaction strategy
  (`4.542798 ms -> 5.406996 ms`).
- Do not retry a standalone exact transaction-control bypass in
  `Connection::execute`. Revisit transaction fixed costs only if a profile
  shows parser/cache lookup inside measured `BEGIN`/`COMMIT` is retained
  self-time and a same-window INSERT section proves lower absolute
  FrankenSQLite medians before any full-matrix run.

## 2026-05-06 - Contiguous repeated-record page-run bulk loader

- Target: remaining tiny/small INSERT gaps after the thread-local parse cache
  and retained writer work, especially `INSERTThroughput — Single Transaction —
  tiny_1col` at 100 rows and small-row page-run fixed costs.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-btree/src/cursor.rs`; the code was briefly committed as
  `bbfc6ddb` and then reverted after the full quick matrix rejected it.
- Candidate shape: split `PendingDirectInsertPageRunRecords::Repeated` into a
  contiguous `RepeatedRange` representation plus a non-contiguous rowid fallback,
  and add a B-tree empty-root bulk loader that accepts
  `(first_rowid, len, repeated_record)` directly. This avoided materializing a
  `Vec<i64>` while buffering monotone rowids and avoided building a temporary
  `Vec<(rowid, payload)>` at flush time for repeated-record page runs.
- Correctness proof passed before measurement:
  `cargo fmt --check`,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-local-profile-target cargo check -p fsqlite-core -p fsqlite-btree --lib`,
  `cargo test -p fsqlite-btree table_bulk_load_empty_root_repeated_record_range -- --nocapture`,
  and
  `cargo test -p fsqlite-core test_prepared_direct_insert_repeated_constant_page_run_flushes_before_read -- --nocapture`.
- Evidence artifacts:
  - Baseline INSERT profile:
    `tests/artifacts/perf/next-profile-crimsongorge-20260506T2253Z/report-insert.json`.
  - Candidate INSERT profile:
    `tests/artifacts/perf/repeated-range-candidate-crimsongorge-20260506T2308Z/report-insert.json`.
  - Candidate full quick:
    `tests/artifacts/perf/repeated-range-candidate-crimsongorge-20260506T2308Z/report-full.json`.
  - Baseline full quick comparator:
    `tests/artifacts/perf/thread-parse-cache-full-repeat-purpleotter-20260506T2228Z/report-full-thread-cache-repeat.json`.
- Result: rejected and reverted. The candidate improved the targeted INSERT
  p99 and write-bulk geomean in the insert-only run (`p99 3.3385 -> 1.8103`,
  write-bulk geomean `1.1322 -> 1.0816`) and moved the worst tiny 100-row
  single-transaction ratio from `3.3385x` to `1.8103x`. The full quick matrix
  still missed the keep gate: primary weighted score regressed
  `0.4328 -> 0.4368`, geomean regressed `0.3341 -> 0.3409`, p90 regressed
  `1.4339 -> 1.4871`, and write-bulk geomean regressed
  `1.1572 -> 1.1679`, despite p99 improving `2.7900 -> 2.5077`.
- Do not retry a standalone repeated-record range representation or repeated
  empty-root bulk-loader as a keep. Reconsider only if paired with a broader
  write-bulk fix that preserves the tiny-row p99 improvement while improving
  the full quick primary score and write-bulk geomean on repeat.

## 2026-05-06 - Engine-side exact benchmark PRAGMA fast path

- Target: fixed setup overhead on the remaining 100-row and 1000-row INSERT
  rows, especially after `profile_fsqlite_insert` showed most measured
  create/begin/prepare/insert/commit phases below the full scenario median.
- Touched during rejected candidate: scratch-only
  `crates/fsqlite-core/src/connection.rs` in
  `/data/tmp/frankensqlite-exact-pragma-purpleotter-20260506T2200Z`; the patch
  was not applied to the main worktree.
- Candidate shape: add an early `Connection::execute` fast path for the exact
  benchmark PRAGMAs:
  `page_size = 4096`, `journal_mode = wal`, `synchronous = normal`,
  `cache_size = -64000`, and
  `fsqlite_capture_time_travel_snapshots=false`. The fast path ran after
  `background_status()` and preserved the pager side effects for journal mode
  and synchronous mode, but skipped parser/planner/VDBE setup for those exact
  statements.
- Correctness/build proof: `cargo fmt -p fsqlite-core --check` passed and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-exact-pragma-target cargo check -p fsqlite-core --lib`
  passed in the scratch worktree.
- Evidence artifacts:
  - Same-window HEAD INSERT section:
    `tests/artifacts/perf/head-insert-samewindow-purpleotter-20260506T220830Z/report-insert-head.json`.
  - Candidate repeat INSERT section:
    `tests/artifacts/perf/exact-pragma-candidate-repeat-purpleotter-20260506T220830Z/report-insert-candidate-repeat.json`.
  - Same-window HEAD full quick:
    `tests/artifacts/perf/head-full-samewindow-purpleotter-20260506T220830Z/report-full-head.json`.
  - Candidate full quick:
    `tests/artifacts/perf/exact-pragma-full-quick-purpleotter-20260506T220830Z/report-full-candidate.json`.
- Result: rejected as below the keep gate. The repeat INSERT section looked
  positive, with weighted score `1.4524 -> 1.2879` and write-bulk geomean
  `1.1859 -> 1.0523`. The full quick matrix was only a tiny overall movement:
  weighted score `0.4484 -> 0.4466`, geomean `0.3485 -> 0.3429`, and p99
  `2.9473 -> 2.7310`, while write-single regressed `1.3798 -> 1.3929`,
  read-single regressed `0.2515 -> 0.2533`, and unrelated rows showed noisy
  absolute FSQLite regressions, including `string-functions` 10K
  `1.480 ms -> 2.517 ms`.
- Do not retry a narrow engine-side exact-PRAGMA bypass as a standalone keep.
  Reconsider only if paired full-quick repeats show a robust matrix win without
  unrelated row regressions, or if benchmark setup is restructured so PRAGMA
  overhead is isolated from workload timing instead of hidden inside every
  scenario.

## 2026-05-06 - Deferred INSERT page-run bulk-load length threshold

- Target: short INSERT rows in the 100-row and 1000-row bands after prior
  page-run work improved 10K medium/large rows but left small/medium short-run
  regressions versus C SQLite.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs`; source was manually reverted after
  the correctness gate failed. The peer-owned dirty
  `crates/fsqlite-btree/src/cursor.rs` file was not edited.
- Candidate shape: add a
  `PREPARED_DIRECT_INSERT_PAGE_RUN_BULK_LOAD_MIN_RECORDS = 2048` threshold so
  deferred page-runs below that length would skip
  `table_bulk_load_empty_root_sorted_records` and replay through the existing
  append path, avoiding the borrowed-record vector and bulk-loader setup for
  short runs. A follow-up repair tried seeding the first non-bulk replay row
  through `table_insert` before appending the rest.
- Correctness evidence: `cargo fmt -p fsqlite-core --check` and
  `git diff --check -- crates/fsqlite-core/src/connection.rs` passed, and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-bulk-threshold-check2 cargo check -p fsqlite-core --lib`
  passed. The focused page-run test failed before benchmarking:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-bulk-threshold-test2 cargo test -p fsqlite-core prepared_direct_insert_page_run -- --nocapture`
  reported
  `test_prepared_direct_insert_page_run_flushes_before_read ... FAILED` with
  `BusySnapshot { conflicting_pages: "page 2184127750 > snapshot db_size 2 (latest: 2)" }`.
- Result: correctness-abandoned and reverted before any benchmark keep gate.
  The failure shows the deferred empty-root page-run flush relies on the bulk
  loader for more than speed: replaying a pending run through the normal append
  APIs from a fresh cursor can corrupt the transaction write set / page image
  enough for the read boundary to observe an impossible page number.
- Do not retry a connection-only "skip bulk load for short pending page-runs"
  threshold. Reconsider only if the B-tree layer grows a proven safe non-bulk
  replay primitive for deferred empty-root runs, with focused visibility,
  savepoint, and rollback tests passing before insert/full-matrix measurement.

## 2026-05-06 - Connection-only prepared INSERT direct record writer

- Target: prepared direct INSERT small-row and record-size rows after the
  insert profile still showed `try_serialize_prepared_direct_simple_insert_record`
  and row serialization cost under `Connection::execute_prepared_direct_simple_insert`.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was manually reverted after measurement. `crates/fsqlite-btree/src/cursor.rs`
  was intentionally left to the concurrent correctness fix on frankensqlite#73.
- Candidate shape: pre-plan each prepared direct INSERT record in `connection.rs`
  and, for small append-shaped rows, feed that plan into the existing
  `table_append_after_last_position_with_writer` path so record bytes are
  written directly into the leaf cell. Larger/fallback rows kept the existing
  `record_scratch` serialization path.
- Correctness proof passed on the candidate before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-direct-writer-check cargo check -p fsqlite-core --lib`
  and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-direct-writer-check cargo test -p fsqlite-core test_prepared_direct_simple_insert -- --nocapture`.
- Evidence artifacts:
  - Baseline:
    `tests/artifacts/perf/insert-section-perf-crimsongorge-20260506T1920Z/report-insert.json`.
  - Candidate:
    `tests/artifacts/perf/direct-writer-insert-crimsongorge-20260506T2008Z/report-insert.json`
    plus sibling `bench.stdout` and `bench.stderr`.
- Result: rejected and reverted. The insert-section weighted score regressed
  from `1.4597` to `1.6675`, geomean regressed from `1.4457` to `1.9495`,
  and write-bulk geomean regressed from `1.4431` to `2.0084`. The target
  `small_3col` 10K single-transaction row regressed from roughly `5.61 ms` in
  the earlier current profile to `11.93 ms`; the record-size `small_3col` 10K
  row measured `12.28 ms`.
- Root-cause read: this saved a record-buffer copy in the connection layer but
  lost more important right-edge locality by entering the existing writer path
  without a retained cached append hint. The hot cost moved into repeated append
  preflight / positioning rather than disappearing.
- Do not retry a connection-only direct record writer. Reconsider only if the
  B-tree exposes a retained cached-hint writer with no duplicate right-edge
  descent, and only keep it after a same-window INSERT-section and full quick
  matrix improvement.

## 2026-05-06 - Batched FSQLite benchmark PRAGMA setup

- Target: 100-row and 1000-row INSERT rows where the profile showed large fixed
  setup cost outside the measured create/begin/prepare/insert/commit phase.
- Touched during rejected candidate:
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`; source was manually
  reverted after the repeat matrix rejected it.
- Candidate shape: change `apply_pragmas_fsqlite` to apply the FSQLite benchmark
  PRAGMAs through one `execute_batch` call, matching the C SQLite harness shape,
  and skip the SQL `PRAGMA page_size` call when the benchmark page size is the
  default already selected by `Connection::open(":memory:")`.
- Correctness/build proof: `cargo fmt -p fsqlite-e2e --check` passed, and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-pragmas-test cargo test -p fsqlite-e2e --bin comprehensive-bench benchmark_pragmas_disable_time_travel_capture -- --nocapture`
  passed before measurement.
- Evidence artifacts:
  - Baseline:
    `tests/artifacts/perf/current-full-baseline-crimsongorge-20260506T1911Z/report-full-baseline.json`.
  - Candidate first run:
    `tests/artifacts/perf/batched-pragmas-crimsongorge-20260506T2030Z/report-full.json`.
  - Candidate repeat:
    `tests/artifacts/perf/batched-pragmas-repeat-crimsongorge-20260506T2032Z/report-full.json`.
- Result: rejected and reverted. The first quick matrix looked mildly positive
  (`primary 0.4809 -> 0.4754`, write-bulk geomean `1.4065 -> 1.3738`), but
  the immediate repeat lost the keep gate: primary score regressed to `0.4846`,
  write-bulk geomean to `1.4511`, write-single geomean to `1.7286`, and p99 to
  `3.0923`.
- Do not retry harness-side PRAGMA batching or default-page-size elision as a
  standalone perf keep. Reconsider only with a same-window baseline/candidate
  pair showing repeatable full-quick improvement, or with an engine-level
  exact-PRAGMA fast path that preserves `execute_pragma` boundary semantics and
  improves the full matrix.

## 2026-05-06 - Follow-up strict CASS negative-result sweep

Scope: user-requested follow-up to mine the last two months of CASS history for
failed, abandoned, reverted, or slower performance ideas without repeating them.
The sweep was scoped to sessions that explicitly map to FrankenSQLite since
`2026-03-06`.

- Search artifact directory:
  `/data/tmp/frankensqlite-cass-ledger-deep-20260506T192044Z`.
- CASS state: `cass 0.4.2`; `cass status` reported a stale but usable index.
  A bounded `timeout 180 cass index --json` refresh was stopped after it stayed
  in `preparing` with `discovered_agents=0` for roughly 36 seconds, matching
  the known stale-index failure mode. The existing index was used with
  CASS-native `view` follow-up.
- Session-set construction:
  - `/data/projects/frankensqlite` explicit path search returned `51`
    sessions.
  - `/dp/frankensqlite` explicit path search returned `26` sessions.
  - exact `--workspace /data/projects/frankensqlite` returned `0` sessions.
  - explicit `/home/ubuntu/.gemini/tmp/frankensqlite` path search returned
    `0` sessions, but broad `frankensqlite` results filtered to source paths
    under `/home/ubuntu/.gemini/tmp/frankensqlite` returned `32`.
  - combined strict de-duplicated set: `95` sessions.
- Negative vocabulary searched through that set included `rejected`,
  `reverted`, `abandoned`, `abandones`, `slower`, `regressed`,
  `didn't help`, `did not help`, `within noise`, `no improvement`,
  `no measurable`, `failed to improve`, `rolled back`, `rollback`,
  `backed out`, `not a keep`, `keep gate`, `not worth keeping`,
  `did not move`, `matrix rejected`, `rejected and reverted`,
  `manually reverted`, `reverted before commit`, `gave up`, `worse`,
  `candidate failed`, `lost to baseline`, `failed the keep gate`,
  `not retry`, `do not retry`, and `do not revive`.
- Useful hit totals included `rejected` (`39`), `reverted` (`29`),
  `abandoned` (`6`), `slower` (`10`), `regressed` (`3`),
  `didn't help` (`6`), `did not help` (`117`), `within noise` (`4`),
  `no improvement` (`219`), `no measurable` (`2`),
  `failed to improve` (`31`), `rolled back` (`11`), `rollback` (`138`),
  `backed out` (`42`), `not a keep` (`37`), `not worth keeping` (`38`),
  `did not move` (`126`), `worse` (`7`), `candidate failed` (`5`), and
  `failed the keep gate` (`18`). Very large `not retry` / `do not retry` /
  `do not revive` counts were mostly self-referential echoes of this ledger and
  recent agent handoffs, not independent evidence.
- High-signal CASS views inspected:
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 120 -C 45`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T06-15-8b4e37ea.json -n 8 -C 35`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-466c7bcd.json -n 150 -C 45`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-80b8129f.json -n 76 -C 45`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-ee1022e3.json -n 30 -C 45`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-52485ea5.json -n 13 -C 35`
- Result: no new distinct artifact-backed performance rejection was found
  beyond entries already in this ledger. This pass adds three concrete
  guardrails:
  - March raw-`bench_insert` serializer/VFS/foldhash/cache summaries are stale
    evidence. They reported only `0.271 s -> 0.265 s` on a raw unique-SQL
    benchmark that intentionally thrashed parse/codegen. Do not use those
    sessions to justify current serializer, SQL-cache, `MemoryFile::write`,
    `PageCache`, `MemPageStore`, or foldhash work without a fresh current
    matrix/profile.
  - Broad Phase-1 optimization-plan summaries mix changes that are already
    present in the current tree, later reverted/public-API-rejected ideas, and
    unmeasured plan text: `Arc` parse/compiled-cache entries, `Arc<VdbeProgram>`
    prepared statements, internal VDBE `SmallVec`, public-row `SmallVec`,
    `execute_params`, prepared-DML VDBE bypass, and IPK `SeekRowid` lowering.
    Do not revive the bundle from CASS prose. Check current code and this
    ledger for the exact subfamily, then measure the one remaining lever.
  - True-asupersync async VFS/Pager/VDBE migration appears in CASS as plan-space
    bead creation, not as a tried-and-rejected micro-optimization. Treat it as
    architecture backlog unless a future session records concrete code and
    benchmark artifacts.
- Retry condition: only add or revive a CASS-derived candidate when
  `cass view`/`cass expand` names a specific code shape and a current commit,
  perf artifact, or correctness-abandonment rationale backs the result. Raw hit
  counts and compaction summaries are triage leads only.

## 2026-05-06 - Direct INSERT text append force-inline annotation

- Target: `comprehensive-bench --quick --filter INSERT` after the clean
  `d9c098ae` record-band page-run commit still showed
  `Connection::append_prepared_direct_simple_insert_text` and
  `Connection::try_serialize_prepared_direct_simple_insert_record` in the
  insert `perf` top symbols.
- Candidate shape: add only `#[inline(always)]` to
  `Connection::append_prepared_direct_simple_insert_text` in
  `crates/fsqlite-core/src/connection.rs`, aiming to remove a hot helper call
  under direct INSERT row construction without changing semantics.
- Evidence artifacts:
  - Fresh-head baseline build/profile:
    `/data/tmp/frankensqlite-purpleotter-head-d9c098ae-profile-20260506T1907Z/insert-profile-head.json`
    and `perf-insert-head-flat.txt`.
  - Candidate insert slice:
    `/data/tmp/frankensqlite-purpleotter-head-d9c098ae-profile-20260506T1907Z/insert-profile-inline-append.json`.
  - Candidate full matrix:
    `/data/tmp/frankensqlite-purpleotter-head-d9c098ae-profile-20260506T1907Z/full-inline-append.json`.
  - Restored full matrix after manual revert:
    `/data/tmp/frankensqlite-purpleotter-head-d9c098ae-profile-20260506T1907Z/full-restored-after-inline-revert.json`.
- Result: rejected and manually reverted. The insert-only profile run improved
  some aggregate insert metrics (`score 1.4715 -> 1.3995`, geomean
  `1.3471x -> 1.3039x`) but worsened the small fixed-cost tail
  (`tiny_1col` 100 rows `2.8769x -> 2.9647x`). The full quick matrix rejected
  the candidate: primary score worsened `0.4883 -> 0.4998`, p99 worsened
  `3.0031x -> 3.2346x`, and the write-single category worsened
  `1.7295x -> 1.7877x`.
- Do not retry standalone `#[inline(always)]` annotations on direct INSERT
  text append/serializer helpers as a perf keep. Reconsider only as part of a
  broader profile-guided compiler-layout pass that proves the full quick matrix
  and small fixed-cost rows improve in the same benchmark window.

## 2026-05-06 - CASS strict-plus-alias failure-vocabulary resweep

Scope: user-requested expansion of this ledger from the last two months of
CASS history, restricted to FrankenSQLite sessions and failure terms such as
`rejected`, `reverted`, `abandoned`, `abandones`, `slower`, `regressed`,
`didn't help`, `did not help`, `within noise`, `no improvement`,
`no measurable`, `failed to improve`, `rolled back`, `backed out`,
`not a keep`, `keep gate`, `not worth keeping`, and `did not move`.

- Search artifact directory:
  `/data/tmp/frankensqlite-cass-ledger-expanded-20260506T172948Z`.
- CASS state: `cass 0.4.2`; `cass status` reported a usable but stale lexical
  index. A capped refresh attempt stayed in `preparing` with no discovered
  agents, so it was stopped and the stale-but-usable index was used with
  CASS-native `view` follow-up.
- Session-set construction:
  - `/data/projects/frankensqlite` explicit path search returned `51`
    sessions.
  - `/dp/frankensqlite` explicit path search returned `26` sessions.
  - exact `--workspace /data/projects/frankensqlite` returned `0` sessions.
  - archived Gemini workspace `/home/ubuntu/.gemini/tmp/frankensqlite`
    returned `97` sessions.
  - broad `/data/projects` workspace plus `frankensqlite` returned `34`
    sessions.
  - combined de-duplicated strict-plus-alias set: `148` sessions.
- Useful hit totals inside that set included `rejected` (`39`), `reverted`
  (`29`), `abandoned` (`6`), `slower` (`10`), `didn't help` (`6`),
  `did not help` (`117`), `regression` (`156`), `rollback` (`138`),
  `no improvement` (`219`), `did not move` (`126`),
  `failed to improve` (`31`), `not a keep` (`37`),
  `not worth keeping` (`38`), and `failed the keep gate` (`18`). The
  misspelling `abandones`, `matrix rejected`, and `rejected and reverted`
  returned no useful hits. The huge `do not retry` / `do not revive` counts
  were self-referential echoes of this ledger and agent summaries, not new
  evidence.
- High-signal CASS views inspected:
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json -n 120 -C 95`
    and `-n 220 -C 75`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-854547a1.json -n 35 -C 60`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 230 -C 80`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-12T00-04-d1f1806b.json -n 260 -C 45`
  - recent multi-repo commit-manager summaries under
    `/home/ubuntu/.claude/projects/-data-projects/16128d2b-9c1f-4615-85ec-babcb706a4a8.jsonl`,
    `/home/ubuntu/.claude/projects/-data-projects/45256a1f-8025-445a-8a4c-4f68bc208028.jsonl`,
    and
    `/home/ubuntu/.claude/projects/-data-projects/09c3f0c0-3833-4514-99e0-0a89c5c41440.jsonl`.
- Result: no new distinct artifact-backed performance reject was found beyond
  the existing explicit entries below. The pass strengthens the existing
  guardrail against broad March "optimize everything" bundles that mixed
  VDBE page-size plumbing, `SmallVec` register/program rewrites, hot register
  helper changes, B-tree seek tweaks, `SqliteValue` `Arc` conversion, and
  benchmark fairness edits while showing stale-file confusion, repeated failed
  replacements, and no same-window matrix proof.
- Do not revive any broad CASS bundle from these hits. Only retry a surviving
  idea after isolating one current code path, proving a current profile signal,
  checking this ledger for that exact family, and running a same-window target
  row plus full-matrix keep gate. Treat commit-manager hits as pointers to
  commits or artifacts only, not as performance evidence.

## 2026-05-06 - Fresh strict CASS failure-vocabulary rerun

Scope: explicit user follow-up to expand this ledger from the last two months
of CASS history, restricted to sessions that clearly map to this project.

- Search artifact directory:
  `/data/tmp/frankensqlite-cass-negative-refresh-20260506T1700Z`.
- Session-set construction:
  - `cass search "/data/projects/frankensqlite" --since 2026-03-06 --robot-format sessions --limit 1000 --mode lexical`
    -> `51` sessions.
  - `cass search "/dp/frankensqlite" --since 2026-03-06 --robot-format sessions --limit 1000 --mode lexical`
    -> `26` sessions.
  - `cass search "frankensqlite" --workspace /data/projects/frankensqlite --since 2026-03-06 --robot-format sessions --limit 1000 --mode lexical`
    -> `0` sessions.
  - Combined strict de-duplicated set: `68` sessions.
- Negative vocabulary searched through that strict set included `rejected`,
  `reverted`, `abandoned`, `abandones`, `slower`, `regressed`,
  `didn't help`, `did not help`, `within noise`, `no improvement`,
  `no measurable`, `failed to improve`, `rolled back`, `rollback`,
  `backed out`, `not a keep`, `keep gate`, `not worth keeping`,
  `did not move`, `matrix rejected`, `rejected and reverted`,
  `manually reverted`, `reverted before commit`, `gave up`, and `worse`.
  Useful totals in this pass included `rejected` (`39`), `reverted` (`29`),
  `abandoned` (`6`), `slower` (`10`), `didn't help` (`6`),
  `did not help` (`117`), `no improvement` (`219`), `rollback` (`138`),
  `not a keep` (`37`), `not worth keeping` (`38`), and
  `did not move` (`126`). The misspelling `abandones`, plus
  `matrix rejected` and `rejected and reverted`, returned no useful strict-set
  hits.
- Focused perf phrases searched included `perf rejected`,
  `benchmark rejected`, `candidate rejected`, `matrix regressed`,
  `weighted score regressed`, `do not retry`, `same-window`,
  `full quick worsened`, `insert matrix worsened`,
  `reverted after measurement`, and `abandoned before benchmark`.
- High-signal views inspected:
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json -n 70 -C 35`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json -n 260 -C 45`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-84f3c374.json -n 36 -C 20`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 1 -C 45`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-1b0c26ee.json -n 1 -C 30`
  - `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-11T04-54-68b4ddee.json -n 190 -C 40`
- Result: no new distinct artifact-backed performance reject was found beyond
  the already-recorded entries below. The useful leads routed back to existing
  guardrails for broad VDBE/`SmallVec`/`SqliteValue` `Arc` rewrites, stale
  raw benchmark evidence, hardcoded/page-size benchmark-policy confusion,
  broad prepared-statement benchmark rewrites, commit-manager summaries, and
  correctness-only audit sessions.
- Important future guardrail: after the ledger expansion, raw CASS hit counts
  are increasingly self-referential because sessions and commit-manager logs
  quote this ledger's own `do not retry` language. Treat counts as triage only;
  record or revive an idea only after `cass view`/`cass expand` identifies a
  concrete candidate and code, commits, or benchmark artifacts back the result.

## 2026-05-06 - VDBE uppercase SUM aggregate sidecar

- Target: `comprehensive-bench --quick --filter Read`
  `Read-After-Write Query Performance`, especially the remaining C-SQLite-win
  row `100 rows / SUM + GROUP BY (~10 groups)`.
- Candidate shape: add an engine-level `FastSumState` sidecar in
  `crates/fsqlite-vdbe/src/engine.rs` for internal uppercase non-DISTINCT
  one-argument `SUM` aggregate opcodes, bypassing aggregate-registry trait
  dispatch while preserving generic/lowercase aggregate behavior.
- Evidence artifacts:
  - Patch snapshot:
    `/data/tmp/frankensqlite-purpleotter-sumfast-20260506T1710Z/sumfast.diff`.
  - First pair:
    `/data/tmp/frankensqlite-purpleotter-sumfast-20260506T1710Z/read-baseline-probe.json`
    and
    `/data/tmp/frankensqlite-purpleotter-sumfast-20260506T1710Z/read-candidate.json`.
  - Same-window repeat:
    `/data/tmp/frankensqlite-purpleotter-sumfast-20260506T1710Z/read-baseline-repeat.json`
    and
    `/data/tmp/frankensqlite-purpleotter-sumfast-20260506T1710Z/read-candidate-repeat.json`.
- Result: rejected and manually reverted before commit. The first pair showed
  only small target-row movement (`1.5219 -> 1.4697`, `0.9395 -> 0.9329`,
  `0.8524 -> 0.8460` for 100/1K/10K row SUM+GROUP BY ratios), and the paired
  repeat did not hold (`1.4569 -> 1.4663`, `0.9368 -> 0.9409`,
  `0.8346 -> 0.8549`). The section weighted score moved slightly in the
  candidate's favor on repeat (`0.24194 -> 0.23863`), but not because the
  targeted remaining gap improved.
- Do not retry a generic engine-level SUM sidecar as a local patch. Revisit
  only if profiling shows aggregate trait dispatch dominates a larger
  C-SQLite-win row and the same-window target rows, not just the section
  aggregate, move in the intended direction.

## 2026-05-06 - Certified direct UPDATE/DELETE pending-run buffer

- Target: `comprehensive-bench --quick --filter update` Section 6
  UPDATE/DELETE rows, where FrankenSQLite remains roughly `1.7x-3.4x` slower
  than legacy C SQLite on the current quick matrix.
- Candidate shape: an alien-artifact / Bε-message-buffer style logical DML
  buffer in `crates/fsqlite-core/src/connection.rs`. Direct monotonic INSERTs
  would certify contiguous rowid presence; later prepared direct-simple
  UPDATE/DELETE calls would return affected counts from that certificate,
  park logical mutations, and flush them at read/savepoint/commit/VDBE
  observation boundaries. The intent was to convert per-row root-to-leaf B-tree
  work into fewer ordered boundary writes while preserving read-your-writes.
- Evidence artifacts:
  - Baseline:
    `tests/artifacts/perf/dml-profile-crimsongorge-20260506T145123Z/report-update-profile.json`.
  - Unrestricted candidate:
    `tests/artifacts/perf/dml-batch-crimsongorge-20260506T153035Z/report-update-dmlbatch.json`.
  - Small certified-table gate:
    `tests/artifacts/perf/dml-batch-smallcert-crimsongorge-20260506T153722Z/report-update-dmlbatch-smallcert.json`.
- Result: rejected and reverted. Baseline Section 6 primary/geomean was
  `2.261040663787221`; the unrestricted candidate worsened to
  `2.6109553897707136`, and the small-certified-table gate still worsened to
  `2.323619620014923`. The parked logical-mutation guard and flush costs
  outweighed any saved B-tree work in the real benchmark shape.
- Do not retry statement-level direct UPDATE/DELETE buffering that flushes via
  repeated direct rowid seeks. Revisit only with a real retained cursor or bulk
  page mutation design that proves a same-window Section 6 improvement.

## 2026-05-06 - Certified direct UPDATE/DELETE scan-merge flush

- Target: the same `comprehensive-bench --quick --filter update` Section 6
  UPDATE/DELETE rows after repeated-seek flushing failed.
- Candidate shape: keep the certified logical DML buffer, but flush a monotonic
  target rowid run by walking the B-tree once from the first key and merging
  queued UPDATE/DELETE mutations as the cursor advances. This tried to turn the
  failed message-buffer into a cache-oblivious scan/merge boundary operation.
- Evidence artifact:
  `tests/artifacts/perf/dml-scanmerge-crimsongorge-20260506T155235Z/report-update-dmlscan.json`.
- Result: rejected and reverted. The scan-merge candidate worsened Section 6
  primary/geomean to `2.5305533708321613` versus the same baseline
  `2.261040663787221`. Small 100-row absolute FSQLite times improved slightly
  (`update 0.333 ms -> 0.309 ms`, `delete 0.310 ms -> 0.288 ms`), but 1K and
  10K rows regressed enough that the actual matrix rejected the idea.
- Do not retry certified direct UPDATE/DELETE scan-merge flushing as a local
  patch. Revisit only if the B-tree layer exposes a true bulk same-page
  mutation path that avoids per-row payload decode/encode and proves both the
  1K/10K rows and Section 6 primary score move.

## 2026-05-06 - B-tree same-size overwrite uncached parse

- Target: `comprehensive-bench --quick --filter update` Section 6
  UPDATE/DELETE, especially direct-simple UPDATE rows where profiles showed
  `CellSlotCache::insert_slow`, `RawVec<CellSlotCacheEntry>::grow_one`,
  `read_cell_pointers_into`, and payload-copy work under
  `BtCursor::table_overwrite_current_payload_same_size_no_overflow`.
- Candidate shape: in `crates/fsqlite-btree/src/cursor.rs`, change
  `overwrite_current_table_payload_same_size_no_overflow` to use
  `parse_cell_at_uncached(top, cell_idx)` instead of
  `parse_cell_at(top, cell_idx)`, avoiding population of the cell-slot cache
  for this one-off overwrite parse. This differs from the earlier cell-slot
  cache rotation experiment: it bypassed cache insertion for the overwrite
  parse rather than changing cache replacement order.
- Behavior proof: scratch
  `cargo test -p fsqlite-btree table_overwrite_current_payload_same_size_no_overflow -- --nocapture`
  passed.
- Evidence artifacts:
  - Scratch worktree:
    `/data/tmp/frankensqlite-purpleotter-scratch-20260506T152149Z` at
    `434e698c36ef6de263616f9349d9ce4db3cdd1bd`.
  - Artifact directory:
    `/data/tmp/frankensqlite-purpleotter-scratch-20260506T152149Z/tests/artifacts/perf/update-uncached-overwrite-purpleotter-20260506T152217Z`.
  - Focused Section 6 baseline/candidate: `update-baseline.json`,
    `update-candidate.json`, plus logs.
  - Full matrix baseline/candidate: `full-baseline.json`,
    `full-candidate.json`, and `full-comparison.txt`.
- Result: rejected and reverted in scratch. A focused profiled Section 6 run
  looked favorable (`write_single` geomean `2.5268x -> 2.2622x`), but the
  unprofiled full quick keep gate rejected it: primary weighted score worsened
  `0.495153 -> 0.501147` (`+1.21%`), average ratio worsened
  `0.885375 -> 0.912127` (`+3.02%`), and p99 worsened
  `3.78475 -> 4.23846` (`+11.99%`). In the full run, UPDATE/DELETE rows were
  mixed and too small to justify keeping: 10K update improved
  (`1.531x -> 1.483x`) but 100-row update/delete and 10K delete regressed.
- Do not retry a standalone `parse_cell_at_uncached` swap in the same-size
  overwrite path. Revisit only if it is fused with a broader overwrite/page
  mutation design that improves same-window full quick after a final
  release-perf rebuild, not just a profiled focused Section 6 run.

## 2026-05-06 - CASS last-two-month failure-vocabulary refresh, current lock pass

Scope: user-requested CASS refresh for failed or abandoned optimization ideas,
restricted to the last two months and sessions that explicitly map to
FrankenSQLite through `/data/projects/frankensqlite`, `/dp/frankensqlite`, or
the exact `/data/projects/frankensqlite` workspace filter.

- Search artifact directory:
  `/data/tmp/frankensqlite-cass-ledger-refresh-20260506T161347Z`.
- Session-set construction:
  - `cass search "/data/projects/frankensqlite" --since=-60d --robot-format sessions --limit 0`
    -> `282` raw paths.
  - `cass search "/dp/frankensqlite" --since=-60d --robot-format sessions --limit 0`
    -> `118` raw paths.
  - `cass search "frankensqlite" --since=-60d --workspace /data/projects/frankensqlite --robot-format sessions --limit 0`
    -> `53` raw paths.
  - Strict de-duplicated set: `346` paths.
  - Broad `frankensqlite` alias set: `718` paths, used only to catch
    archived/alias false negatives.
- Negative vocabulary searched through the session set included `rejected`,
  `reverted`, `abandoned`, `abandones`, `slower`, `regressed`, `didn't help`,
  `did not help`, `within noise`, `no improvement`, `no measurable`,
  `failed to improve`, `rolled back`, `rollback`, `backed out`, `not a keep`,
  `keep gate`, `matrix rejected`, `rejected and reverted`,
  `manually reverted`, `reverted before commit`, and `not worth keeping`.
- Raw counts were large, but inspection did not identify a new older
  artifact-backed performance reject beyond entries already represented in this
  ledger. High-signal hits routed back to existing no-retry families: broad
  VDBE/`SmallVec`/`SqliteValue` rewrites, stale raw `bench_insert` work,
  prepared-DML bypasses, direct INSERT/DELETE micro-candidates, WAL
  publication/checksum experiments, and benchmark-methodology rejects.
  Multi-repo commit summaries and correctness-only reviews were excluded.
- Follow-up strict-path recount after the ledger lock was released used the
  stale-but-usable CASS `0.4.2` index and wrote artifacts under
  `/data/tmp/frankensqlite-cass-ledger-refresh-20260506T1624`. The exact
  workspace filter returned `0` sessions, but explicit path aliases yielded a
  `64`-session strict set. Useful hit totals inside that set included
  `rejected` (`32`), `reverted` (`27`), `abandoned` (`6`), `slower` (`10`),
  `didn't help` (`6`), `did not help` (`103`), `rollback` (`127`),
  `not a keep` (`37`), `not worth keeping` (`34`), and `did not move` (`112`);
  `abandones`, `matrix rejected`, and `rejected and reverted` returned no
  useful hits. Targeted `cass view` inspection again routed the high-signal
  hits to already-recorded broad-March-bundle and commit-manager guardrails
  rather than a new distinct benchmark-backed no-retry item.
- Future agents should keep using a project session set plus CASS-native
  follow-up (`cass view`, `cass expand`, or `cass export`), because raw
  `source_path`s may be archived and exact workspace filters remain
  sparse/noisy.

## 2026-05-06 - Direct `std::thread` launch in comprehensive concurrent benchmark

- Target: `comprehensive-bench --quick --filter concurrent`, after a perf
  sample of the dirty page-run tree showed the concurrent filter dominated by
  thread creation / `asupersync::RuntimeBuilder` setup rather than MVCC work.
- Candidate shape: in `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`,
  replace the measured concurrent section's `RuntimeBuilder` + `spawn_blocking`
  helper with a direct `std::thread::spawn` wrapper for both the C SQLite and
  FrankenSQLite arms. This was a benchmark-harness parity attempt, not an
  engine change. Source was manually reverted after measurement and was not
  applied to the shared checkout.
- Evidence artifacts:
  - Baseline/candidate A/B:
    `/data/tmp/frankensqlite-purpleotter-stdthread-ab-20260506T182458Z/baseline-concurrent.json`
    and
    `/data/tmp/frankensqlite-purpleotter-stdthread-ab-20260506T182458Z/candidate-concurrent.json`.
  - Pre-candidate perf lead:
    `/data/tmp/frankensqlite-purpleotter-concurrent-profile-20260506T181221Z/perf-concurrent-flat.txt`
    and
    `/data/tmp/frankensqlite-purpleotter-concurrent-profile-20260506T181221Z/perf-concurrent-children.txt`.
- Result: rejected and manually reverted. The candidate made both engines
  faster, but C SQLite benefited more at low concurrency, so the concurrent
  score worsened: baseline score `0.7431861039` versus candidate
  `0.8397287843`. Row detail: `2 writers x 1000 rows` ratio worsened
  `1.0286 -> 1.7133`; `4 writers x 1000 rows` worsened `1.0266 -> 1.0384`;
  only `8 writers x 1000 rows` improved `0.3887 -> 0.3328`.
- Do not retry replacing the concurrent section runtime wrapper as a standalone
  perf keep. Revisit only if the benchmark methodology changes to exclude
  thread-launch/setup from both engines and the full quick matrix score
  improves in the same window.

## 2026-05-06 - B-tree bulk page direct pointer writes

- Target: `comprehensive-bench --quick --filter insert`, especially
  page-run-backed INSERT rows where the empty-root bulk page builder allocates
  a temporary `cell_offsets` vector, fills it, then writes the pointer array
  into the page.
- Candidate shape: in `crates/fsqlite-btree/src/cursor.rs`, write each
  leaf/interior cell pointer directly into the destination page while laying
  out bulk table cells, removing the temporary `Vec<u16>` and final
  `cell::write_cell_pointers` call. The candidate was manually reverted and
  not committed.
- Evidence artifacts:
  - Current-main insert baseline:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T164206Z/insert-current.json`
    and `insert-current.out`.
  - Current-main full baseline:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T164206Z/full-current.json`
    and `full-current.out`.
  - Candidate insert slice:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T164206Z/insert-direct-pointers.json`
    and `insert-direct-pointers.out`.
  - Candidate full quick:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T164206Z/full-direct-pointers.json`
    and `full-direct-pointers.out`.
- Result: rejected and reverted. The insert slice worsened on the broad
  section gates: average ratio `1.6577 -> 1.6919`, geomean
  `1.6073 -> 1.6519`, and write-bulk average `1.6674 -> 1.7199`.
  It did improve the partial weighted insert score (`1.5912 -> 1.5295`),
  but that came with a severe large-row regression, including
  `large_10col` 10K single-transaction FSQLite median `13.30 ms -> 21.38 ms`
  in the profiled insert slice. The full candidate run was also contaminated
  by concurrent unowned `connection.rs` edits, so it is not valid keep evidence
  for the pointer-write change.
- Do not retry plain direct pointer writes in the bulk page builders as a
  standalone optimization. Revisit only with an isolated same-window A/B and a
  design that preserves the optimized chunked pointer write behavior while
  proving both the insert section and full quick matrix improve.

## 2026-05-06 - Direct INSERT rowid-presence certificate no-op diagnostic

- Target: `comprehensive-bench --quick --filter insert`, especially fixed-cost
  direct INSERT rows after the current dirty-tree insert profile showed
  rowid-presence certification and direct-record serialization still visible in
  the prepared direct INSERT path.
- Candidate shape: in a scratch worktree based on current dirty state, turn the
  direct rowid-presence certificate update/check into a diagnostic no-op to
  measure whether the certificate machinery was worth optimizing or disabling.
  Source was reverted after measurement and was never applied to the shared
  checkout.
- Evidence artifacts:
  - Scratch worktree:
    `/data/tmp/frankensqlite-purpleotter-cert-scratch-20260506T155132Z`.
  - Artifact directory:
    `/data/tmp/frankensqlite-purpleotter-cert-art-20260506T155132Z`.
  - Baseline comparator:
    `/data/tmp/frankensqlite-purpleotter-current-refresh-20260506T154144Z/insert-profile-current-repeat.json`.
  - Candidate: `insert-cert-noop.json`, `insert-cert-noop.log`,
    `build-cert-noop.log`, and `cert-noop-diagnostic.diff` in the artifact
    directory.
- Result: rejected and reverted in scratch. Insert score worsened
  `1.4853 -> 1.5042` (`+1.27%`), average ratio worsened `1.5605 -> 1.5901`,
  and geomean worsened `1.4944 -> 1.5631`.
- Do not retry disabling or bypassing the direct INSERT rowid-presence
  certificate as a standalone perf win. Revisit only if a current profile
  proves certificate maintenance dominates and the same-window insert matrix
  improves.

## 2026-05-06 - `write_varint` 2/3-byte encoder fast path

- Target: prepared direct INSERT record serialization after the dirty-tree
  insert profile still showed row-build/record serialization cost and direct
  INSERT calls `write_varint` for header sizes and serial types.
- Touched during rejected candidate: `crates/fsqlite-types/src/serial_type.rs`;
  source was manually reverted after measurement.
- Candidate shape: add explicit 1-, 2-, and 3-byte branches to `write_varint`,
  falling back to the existing loop only for 4-9 byte varints. This aimed to
  remove `varint_len()` plus the reverse loop from common text/blob serial
  types in medium/large direct INSERT rows.
- Correctness proof:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-varint cargo test -p fsqlite-types varint -- --nocapture`
  passed before the A/B (`33 passed`).
- Evidence artifacts:
  `/data/tmp/frankensqlite-purpleotter-varint-ab-20260506T183709Z/baseline-insert.json`
  and
  `/data/tmp/frankensqlite-purpleotter-varint-ab-20260506T183709Z/candidate-insert.json`.
- Result: rejected and manually reverted. Insert primary weighted score moved
  only `1.438533 -> 1.431092`, while geomean worsened
  `1.401992 -> 1.444521`, C-wins increased `19 -> 23`, and
  FrankenSQLite-wins dropped `4 -> 1`. Row-level swings were too noisy and
  included major tiny-row regressions, so the small primary-score improvement
  is not a keep.
- Retry condition: only reconsider with a focused microbenchmark or perf sample
  proving `write_varint` itself is a top retained self-time hotspot, followed
  by a same-window insert and full quick matrix improvement. Do not retry this
  as a standalone direct INSERT lever.

## 2026-05-06 - Direct INSERT float-multiply evaluator fast path

- Target: `comprehensive-bench --quick --filter insert`, especially
  text/numeric expression direct INSERT rows where row-build and expression
  evaluation still show up under
  `Connection::try_serialize_prepared_direct_simple_insert_record`.
- Candidate shape: in the current-dirty scratch worktree, add a narrow
  evaluator fast path for the benchmark-shaped floating multiply expression
  used by direct INSERT row construction. Source was reverted after measurement
  and was never applied to the shared checkout.
- Evidence artifacts:
  - Scratch worktree:
    `/data/tmp/frankensqlite-purpleotter-cert-scratch-20260506T155132Z`.
  - Artifact directory:
    `/data/tmp/frankensqlite-purpleotter-cert-art-20260506T155132Z`.
  - Baseline comparator:
    `/data/tmp/frankensqlite-purpleotter-current-refresh-20260506T154144Z/insert-profile-current-repeat.json`.
  - Candidate: `insert-float-mul-fastpath.json`,
    `insert-float-mul-fastpath.log`, `build-float-mul-fastpath.log`, and
    `float-mul-fastpath.diff` in the artifact directory.
- Result: rejected and reverted in scratch. Insert score worsened
  `1.4853 -> 1.5877` (`+6.89%`), average ratio worsened `1.5605 -> 1.6386`,
  geomean worsened `1.4944 -> 1.5929`, and many FSQLite medians rose.
- Do not retry standalone numeric-expression micro-specialization for this
  direct INSERT path. Revisit only as part of a fused prepared
  expression/template/page-builder design that improves absolute target medians
  and the weighted insert score in the same run.

## 2026-05-06 - Statement microbatch schema-validation window `max_r=128`

- Target: `comprehensive-bench --quick --filter insert`, especially prepared
  direct INSERT rows where `schema_validation_ns` remained visible and prior
  CASS/ledger entries showed smaller statement-renewal microbatch ideas stayed
  noisy.
- Candidate shape: in the current-dirty scratch worktree, raise the prepared
  statement microbatch row window from `16` to `128` to reduce repeated schema
  validation and renewal work across larger INSERT runs. Source was reverted
  after measurement and was never applied to the shared checkout.
- Evidence artifacts:
  - Scratch worktree:
    `/data/tmp/frankensqlite-purpleotter-cert-scratch-20260506T155132Z`.
  - Artifact directory:
    `/data/tmp/frankensqlite-purpleotter-cert-art-20260506T155132Z`.
  - Baseline comparator:
    `/data/tmp/frankensqlite-purpleotter-current-refresh-20260506T154144Z/insert-profile-current-repeat.json`.
  - Candidate: `insert-microbatch-r128.json`, `insert-microbatch-r128.log`,
    `build-microbatch-r128.log`, and `microbatch-r128.diff` in the artifact
    directory.
- Result: rejected and reverted in scratch. The candidate slightly reduced
  `schema_validation_ns` on 10K `small_3col` rows (`316751 -> 294791` and
  `316823 -> 288075` in sampled rows), but the actual insert matrix worsened:
  score `1.4853 -> 1.5386` (`+3.59%`), average ratio `1.5605 -> 1.6097`, and
  geomean `1.4944 -> 1.5749`.
- Do not retry merely raising the prepared statement microbatch row window as a
  standalone INSERT optimization. Revisit only if schema validation becomes a
  dominant current profile cost and a same-window matrix proves the larger
  window helps more than it hurts.

## 2026-05-06 - Bulk-load root-page fit precheck fast-fail

- Target: `comprehensive-bench --quick --filter insert` and full
  `comprehensive-bench --quick`, especially bulk insert rows where the empty
  root sorted-record bulk loader first checked whether all records fit on the
  root page before regrouping into leaf pages.
- Candidate shape: in `crates/fsqlite-btree/src/cursor.rs`, replace the
  initial root-page `bulk_table_leaf_groups(records, root_header_offset)` call
  with a `bulk_table_leaf_fits_one_page` boolean precheck that can stop as soon
  as the first root page overflows, avoiding construction of a throwaway group
  vector over the rest of large record runs. Source was reverted after the
  full-matrix keep gate failed.
- Evidence artifacts:
  - Baseline/current full quick:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T1249Z/full-current.json`.
  - Same-window restored insert baseline:
    `/data/tmp/frankensqlite-purpleotter-bulk-root-fit-20260506T1304Z/insert-restored-baseline.json`.
  - Candidate insert probes:
    `/data/tmp/frankensqlite-purpleotter-bulk-root-fit-20260506T1304Z/insert-candidate-repeat.json`
    and
    `/data/tmp/frankensqlite-purpleotter-bulk-root-fit-20260506T1304Z/insert-candidate-second.json`.
  - Candidate full quick:
    `/data/tmp/frankensqlite-purpleotter-bulk-root-fit-20260506T1304Z/full-candidate.json`
    and final post-clippy rebuild
    `/data/tmp/frankensqlite-purpleotter-bulk-root-fit-20260506T1304Z/full-final.json`.
- Result: rejected and reverted. Insert-only repeats were noisy and looked
  slightly favorable (`1.539 -> 1.528` and `1.539 -> 1.483` weighted insert
  score in two comparisons), but the final full quick matrix worsened versus
  current baseline: weighted score `0.4854 -> 0.4918`, average ratio
  `0.7805 -> 0.8348`, and write_bulk average `1.6736 -> 1.9103`.
- Do not retry an isolated root-page fit precheck. Revisit only if the larger
  bulk-loader design removes the duplicate page-planning pass entirely or fuses
  grouping with page construction, and keep it only if same-window full quick
  improves after a final release-perf rebuild.

## 2026-05-06 - Skipping default page-size PRAGMAs in benchmark setup

- Target: `comprehensive-bench --quick --filter insert`, especially short
  write rows where setup/open/PRAGMA overhead dominates.
- Candidate shape: in
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`, avoid executing
  `PRAGMA page_size = 4096` for both C SQLite and FrankenSQLite when
  `FSQLITE_BENCH_PAGE_SIZE` is the default, while preserving the non-default
  env override path.
- Evidence artifacts:
  - Baseline/current:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T1249Z/insert-current.json`.
  - Candidate:
    `/data/tmp/frankensqlite-purpleotter-skip-default-pagesize-20260506T1255Z/insert-candidate.json`,
    `insert-candidate.log`, and `candidate.diff`.
- Result: rejected and reverted. The insert average ratio worsened from
  `1.703x` to `1.74x`; several noisy fixed-cost rows moved in opposite
  directions, and `100 rows / autocommit` small_3col regressed sharply
  (`199.4 us -> 351.5 us` for FrankenSQLite in same-host quick runs).
- Do not retry benchmark default-page-size PRAGMA elision as a performance
  optimization. Revisit only if benchmark methodology changes explicitly remove
  default page-size setup from both engines and the full quick matrix improves,
  not just one setup-dominated row.

## 2026-05-06 - Broad direct-INSERT page-run activation thresholds

- Target: `INSERTThroughput --quick --filter insert`, especially
  single-transaction and record-size `large_10col` 10K rows.
- Candidate shape: Bε-tree-style pending page-run buffering in
  `crates/fsqlite-core/src/connection.rs` plus empty-root sorted-record bulk
  loading in `crates/fsqlite-btree/src/cursor.rs`, but with broad activation
  thresholds that also caught medium/small record shapes.
- Evidence artifacts:
  - Unthresholded/broad:
    `tests/artifacts/perf/page-run-bulk-crimsongorge-20260506T112823Z/report-insert-quick.json`
    and `report-insert-quick-noprofile.json`.
  - `96` byte threshold:
    `tests/artifacts/perf/page-run-bulk-crimsongorge-20260506T112823Z/report-insert-quick-threshold96.json`
    and `report-insert-quick-threshold96-repeat.json`.
- Result: rejected as a broad activation policy, not as a page-run primitive.
  The unthresholded candidate worsened the insert matrix, and the `96` byte
  threshold looked promising once but failed the repeat gate: average ratio
  `1.750x -> 1.820x`, geomean `1.692x -> 1.775x`, and weighted score
  `1.496 -> 1.652`. Do not activate this primitive for medium/small records:
  per-row guard/buffer costs and delayed flush overhead swamp the avoided
  B-tree work there.
- Revisit only with a narrow large-record gate or a record-template/page
  builder fusion that proves both absolute `large_10col` medians and the
  weighted insert score in same-window repeat runs.

## 2026-05-06 - Medium page-run admission over-broad arena bands

- Target: `comprehensive-bench --quick --filter insert` after the active
  prepared-direct INSERT page-run append fast path, especially
  single-transaction and record-size `medium_6col` 10K rows.
- Candidate shape: lower the prepared direct INSERT page-run admission gate
  from `512` bytes to `128` bytes in `crates/fsqlite-core/src/connection.rs`,
  and teach `crates/fsqlite-btree/src/cursor.rs` to bulk-load borrowed
  records so a pending page-run can store medium payloads in one contiguous
  arena. Rejected variants included:
  - `128` byte admission with owned `Vec<u8>` records for all admitted runs.
  - borrowed arena records for all admitted runs up to `512` bytes.
  - borrowed arena records only below `256` bytes.
- Evidence artifacts:
  - Owned `128` threshold:
    `tests/artifacts/perf/page-run-threshold128-active-append-crimsongorge-20260506T171751Z/report-insert-threshold128-active-append.json`
    and repeat
    `tests/artifacts/perf/page-run-threshold128-active-append-repeat-crimsongorge-20260506T172245Z/report-insert-threshold128-active-append-repeat.json`.
  - All-arena `128..512` threshold:
    `tests/artifacts/perf/page-run-borrowed-threshold128-crimsongorge-20260506T173312Z/report-insert-borrowed-threshold128.json`
    and full-matrix check
    `tests/artifacts/perf/page-run-hybrid-threshold128-full-crimsongorge-20260506T175132Z/report-full-hybrid-threshold128.json`.
  - Too-narrow arena cap:
    `tests/artifacts/perf/page-run-hybrid-threshold128-arena256-insert-crimsongorge-20260506T175709Z/report-insert-hybrid-threshold128-arena256.json`.
- Result: rejected as over-broad arena/admission bands, not as the final
  record-band policy. The owned `128` threshold and all-arena variants made
  medium rows faster but regressed large rows sharply: for example the
  all-arena full matrix pushed record-size `large_10col` to
  `20.65 ms` / `2.22x` versus C SQLite. The `256` byte arena cap protected
  large rows better, but lost too much of the medium single-transaction win
  (`medium_6col` 10K single transaction stayed `1.41x` slower than C SQLite).
- Do not retry broad `128` admission or all-arena page-run buffering as a
  standalone optimization. The only measured keepable form from this pass is
  record-band isolation: page-run admission at `128` bytes, arena storage below
  `384` bytes, and the existing owned-record storage above that cap, with both
  insert-only and full quick matrix proof.

## 2026-05-06 - Prepare-time direct-INSERT record template

- Target: fixed-cost and row-build-heavy `INSERTThroughput --quick --filter
  insert` rows after the direct-record serializer still left row construction
  work in `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: an alien-artifact-style prepare-time record template for
  benchmark-shaped direct INSERTs. The template recognized simple integer
  parameter arithmetic and concat chains at prepare time, then wrote SQLite
  record headers/bodies directly without building the per-row
  `PreparedDirectInsertRecordValue` and layout `SmallVec`s. Unsupported runtime
  parameter types fell back to the existing direct serializer.
- Evidence artifacts:
  - Baseline/kept predecessor:
    `tests/artifacts/perf/fixed-cost-write-crimsongorge-20260506T1225Z/report-insert-open-skip.json`
    and `bench-insert-open-skip.stderr`.
  - Candidate:
    `tests/artifacts/perf/record-template-crimsongorge-20260506T1328Z/report-insert-template.json`
    and `bench-insert-template.stderr`.
- Result: rejected and manually reverted. A few tiny/small fixed-cost rows
  improved, but the insert matrix worsened: weighted score `1.5646 -> 1.5825`,
  average ratio `1.6709 -> 1.7507`, geomean `1.6319 -> 1.7102`, and write-bulk
  average `1.6883 -> 1.7797`. The candidate made medium/large row-build worse:
  for example record-size `large_10col` FSQLite median worsened from about
  `14.35 ms` to `18.36 ms`, and profiled `large_10col` row-build rose to about
  `8.42 ms`.
- Do not retry a separate per-row record-template interpreter. Revisit this
  idea only if it is fused with a bulk page-run/page builder that computes
  record bodies and page layout in one pass over many rows, or if a same-window
  benchmark proves the template lowers absolute medium/large FSQLite medians
  and the weighted insert score.

## 2026-05-06 - Connection-level exact control-statement fast path

- Target: setup/open-heavy fixed-cost write rows in
  `comprehensive-bench --quick --filter insert`, where `BEGIN`, `COMMIT`, and
  benchmark PRAGMAs are repeatedly executed through the general SQL
  parser/planner/VDBE path.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, recognize exact
  single-statement control SQL (`BEGIN`, `COMMIT`, `ROLLBACK`, savepoint
  boundaries, and known benchmark PRAGMAs) at `Connection::execute` after
  background-status polling, then route directly to the corresponding
  transaction or pragma helpers without preparing a statement.
- Evidence artifacts:
  - Baseline/kept predecessor:
    `tests/artifacts/perf/fixed-cost-write-crimsongorge-20260506T1225Z/report-insert-open-skip.json`
    and
    `tests/artifacts/perf/fixed-cost-write-crimsongorge-20260506T1225Z/report-full-open-skip.json`.
  - Candidate:
    `tests/artifacts/perf/fixed-cost-write-crimsongorge-20260506T1302Z/report-insert-control-fast.json`
    and
    `tests/artifacts/perf/fixed-cost-write-crimsongorge-20260506T1302Z/report-full-control-fast.json`.
- Result: rejected and manually reverted. The insert-only section improved
  (`weighted 1.5646 -> 1.4820`, p99 `3.3446 -> 2.3417`), but the full quick
  matrix worsened (`weighted 0.4871 -> 0.4953`, p99 `2.9898 -> 3.6056`). The
  improvement was too narrow and the global tail regression failed the keep
  gate.
- Do not retry a broad `Connection::execute` exact-control fast path as an
  isolated optimization. Revisit only with per-call-site preparation reuse or a
  control-plane automaton that proves full quick weighted and p99 scores both
  improve in the same run.

## 2026-05-06 - Quotient-filter inactive maintenance fast-null bit

- Target: `comprehensive-bench --quick --filter insert`, especially direct
  INSERT rows where every successful row currently calls the dormant
  quotient-filter maintenance hook in `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: add a `quotient_filters_active: Cell<bool>` fast-null bit to
  skip the per-row `RefCell<HashMap<...>>` borrow/probe in `qf_record_insert`
  and `qf_record_delete` when no quotient-filter entries exist. The intent was
  to remove a dormant probabilistic-filter side cost from benchmark-shaped
  INSERT loops without changing behavior when a filter entry is present.
- Evidence artifacts:
  - Baseline:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T132837Z/insert-profile-current.json`
    and `insert-profile-current.log`.
  - Candidate:
    `tests/artifacts/perf/qf-guard-crimsongorge-20260506T1358Z/report-insert-qf-guard.json`
    and `bench-insert-qf-guard.stderr`.
- Result: rejected and manually reverted. Large rows improved in the candidate
  run (`large_10col` 10K single-txn ratio `1.896 -> 1.524`, record-size
  `large_10col` ratio `1.780 -> 1.493`), but the insert section worsened
  overall: average ratio `1.696 -> 1.709`, geomean `1.653 -> 1.672`, median
  `1.590 -> 1.635`. Small/medium rows regressed enough to fail the section
  keep gate, including `small_3col` 10K `1.352 -> 1.635` and `medium_6col`
  1000 rows `1.653 -> 1.959`.
- Do not retry a standalone inactive quotient-filter guard based only on the
  dormant-hook theory. Revisit only if a same-window profile isolates QF
  maintenance as a top contributor and a full insert or full quick matrix shows
  the small/medium rows neutral while preserving the large-row improvement.

## 2026-05-06 - Arena-backed direct INSERT page-run buffer

- Target: `comprehensive-bench --quick --filter insert`, especially the
  large-record page-run path where the current pending page-run stores each
  serialized record as its own `Vec<u8>` before the empty-root B-tree bulk
  loader lays out leaf pages.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, replace
  `Vec<(rowid, Vec<u8>)>` with one contiguous payload arena plus row metadata,
  then add a borrowed-record bulk-loader entry point in
  `crates/fsqlite-btree/src/cursor.rs`. This was the small version of the
  Bε-tree/region-allocation idea: avoid per-row `Vec` allocations while keeping
  the existing strict monotone rowid, empty-root, no-overflow fallback envelope.
- Evidence artifacts:
  - Baseline:
    `/data/tmp/frankensqlite-purpleotter-current-20260506T132837Z/insert-profile-current.json`
    and `insert-profile-current.log`.
  - Candidate:
    `tests/artifacts/perf/page-run-arena-crimsongorge-20260506T141801Z/report-insert-page-run-arena.json`
    and `bench-insert-page-run-arena.stderr`.
- Result: rejected and manually reverted. The candidate improved one targeted
  large row (`large_10col` 10K single transaction FSQLite mean `18.19 ms ->
  15.82 ms`, ratio `1.896 -> 1.515`), but did not improve the section keep
  gate: weighted insert score worsened `1.5467 -> 1.5851`, average ratio
  `1.696 -> 1.740`, and geomean `1.653 -> 1.709`. The record-size
  `large_10col` absolute FSQLite time also worsened `17.32 ms -> 17.93 ms`,
  so the improvement was not stable enough to justify the extra bulk-loader
  API surface.
- Do not retry an arena-only page-run buffer as the standalone next step.
  Revisit only with true fused record-body and page-layout construction over a
  run, where the benchmark proves both absolute `large_10col` medians and the
  weighted insert/full-quick scores improve in a same-window run.

## 2026-05-06 - Larger page-size geometry as standalone record-size INSERT fix

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially the `large_10col` row that remains far behind legacy C SQLite.
- Candidate shape: benchmark-only page-size control in
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`, keeping C SQLite and
  FrankenSQLite on the same page size via `FSQLITE_BENCH_PAGE_SIZE`. Tested
  default `4096`, `8192`, and `16384` byte pages under the same release-perf
  binary with `FSQLITE_BENCH_PROFILE_INSERT=1`.
- Evidence artifacts:
  - Default:
    `tests/artifacts/perf/page-size-record-crimsongorge-20260506T0916Z-default/report.json`
    and `bench.log`.
  - `8192`:
    `tests/artifacts/perf/page-size-record-crimsongorge-20260506T0916Z-8192/report.json`
    and `bench.log`.
  - `16384`:
    `tests/artifacts/perf/page-size-record-crimsongorge-20260506T0916Z-16384/report.json`
    and `bench.log`.
- Result: rejected as a standalone large-row fix. The geometry lever was real
  for `medium_5col`: default FSQLite `11.56 ms` / `2.22x` became `7.04 ms` /
  `1.42x` at `16384`, with quick-balance attempts dropping from `453` to
  `111`. It did not close `large_10col`: default FSQLite `30.91 ms` / `3.21x`
  became `25.64 ms` / `2.99x` at `16384`, while C SQLite also improved
  `9.62 ms -> 8.57 ms`. The profile showed fewer splits but persistent
  page-size-sensitive costs (`btree_insert_ns` around `6.58 ms` and
  `commit_roundtrip_ns` around `7.13 ms` at `16384`).
- Do not make larger page size the engine default or present it as the step
  change. Revisit page geometry only as part of a true monotonic page-run
  builder that fills leaf pages and parent dividers in bulk, or after the
  per-split/commit write-set page-image cost is removed.

## 2026-05-06 - Atomic published-page slot floor reduction

- Target: `comprehensive-bench --quick --filter concurrent`, especially the
  2-writer small-row gap where connection-open/setup cost looked like a possible
  component.
- Candidate shape: lower `ATOMIC_PUBLISHED_MIN_SLOT_COUNT` in
  `crates/fsqlite-pager/src/pager.rs` from `512` to `128` in a scratch
  worktree, reducing fixed `PublishedPagerState::new` allocation and mutex
  initialization for open-heavy workloads. Source was restored after
  measurement.
- Evidence artifacts:
  - Scratch worktree:
    `/data/tmp/frankensqlite-purpleotter-profile-20260506T074017Z`.
  - Reports:
    `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/mt2-250-baseline-repeat.json`,
    `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/mt2-250-slot128.json`,
    `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/mt-250-slot128-repeat.json`,
    `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/comprehensive-concurrent-slot128.json`,
    and
    `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/comprehensive-concurrent-baseline-repeat.json`.
- Result: rejected. The focused 2-writer micro-harness improved
  `3.00 ms -> 2.77 ms` for FSQLite and ratio `1.62x -> 1.51x`, but the actual
  comprehensive concurrent same-window section did not improve overall:
  candidate section average was `0.87x` versus restored baseline `0.85x`, with
  the 8-writer row worsening (`41.73 ms` candidate versus `37.65 ms` restored
  baseline).
- Do not retry lowering the atomic published-page slot floor as an isolated
  open-cost optimization. Revisit only with a design that preserves or improves
  the 8-writer comprehensive row and the section average.

## 2026-05-06 - Direct payload quick-balance row-cell transfer

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially `large_10col`, where the profile showed about `2001` right-edge
  quick-balance attempts and `4004` table leaf cell-assembly calls.
- Candidate shape: in `crates/fsqlite-btree/src/balance.rs` and
  `cursor.rs`, add a no-overflow quick-balance variant that builds the new
  right sibling directly from the row payload (`rowid + record`) instead of
  first assembling a temporary table-leaf cell buffer and copying that cell into
  the new page. The fallback path still used the existing encoded-cell balance
  for overflow rows, parent-full cases, and conservative reloads. Source was
  reverted after measurement.
- Correctness/build proof before measurement: `cargo fmt --check -p
  fsqlite-btree`, `cargo check -p fsqlite-btree --lib`,
  `cargo clippy -p fsqlite-btree --lib -- -D warnings`, and
  `cargo test -p fsqlite-btree cached_rightmost_leaf_hint -- --nocapture`
  passed.
- Evidence artifacts:
  - Baseline/page-size probe default:
    `tests/artifacts/perf/page-size-record-crimsongorge-20260506T0916Z-default/report.json`
    and `bench.log`.
  - Candidate:
    `tests/artifacts/perf/direct-payload-qb-crimsongorge-20260506T1030Z-candidate/report.json`,
    `report-repeat.json`, and `bench.log`.
- Result: rejected. The candidate did not reduce table leaf cell assembly
  counts and made quick balance materially slower. In the repeat candidate run,
  `large_10col` measured C SQLite `9.46 ms` versus FSQLite `31.30 ms`
  (`3.31x`), while baseline default was FSQLite `30.91 ms` / `3.21x`.
  Profile counters worsened on the target path:
  `btree_quick_balance_ns` rose from about `2.54 ms` to `5.17 ms`, and
  `btree_insert_ns` rose from about `7.14 ms` to `10.22 ms`.
- Do not retry direct row-payload quick-balance as a standalone row-cell
  transfer. The extra parent probe/restore path and duplicated split decision
  dominate any avoided temporary-cell copy. Revisit only inside a real
  monotonic page-run builder that makes one split decision for many rows and
  emits parent dividers in bulk.

## 2026-05-06 - Concat-chain text-piece direct-record transducer

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially `large_10col`, after the fused direct-record serializer removed
  `serialize_ns` but still left about `6.35 ms` in direct INSERT row-build
  work.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, add a
  `PreparedDirectInsertRecordValue::TextPieces` lane and supporting borrowed /
  integer / float concat-piece helpers so compiled concat chains could compute
  TEXT lengths and write pieces directly into the record body instead of first
  materializing each concat result in the reusable text scratch. Source was
  reverted after measurement.
- Correctness/build proof before measurement: `cargo fmt --check -p
  fsqlite-core`, `cargo check -p fsqlite-core --lib`, `cargo clippy -p
  fsqlite-core --lib -- -D warnings`,
  `cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`,
  and
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`
  passed.
- Evidence artifacts:
  - Candidate:
    `tests/artifacts/perf/concat-transducer-crimsongorge-20260506T1124Z/report-insert-quick.json`
    and `bench-insert-quick.log`.
  - Kept fused-record predecessor:
    `tests/artifacts/perf/fused-record-crimsongorge-20260506T100113Z/report-insert-quick-repeat.json`
    and `bench-insert-quick-repeat.log`.
- Result: rejected. Candidate quick insert summary worsened versus the fused
  predecessor: average ratio `1.789x -> 1.818x`, geomean `1.724x -> 1.765x`,
  and write-bulk average `1.826x -> 1.864x`. `large_10col` ratio looked better
  only because C SQLite was slower in the candidate run; FSQLite absolute time
  worsened `28.20 ms -> 30.82 ms`. `small_3col` also regressed sharply
  `4.40 ms -> 5.73 ms`. Profile still showed `row_build_ns` around `6.41 ms`
  for record-size `large_10col`, so extra per-row piece bookkeeping did not
  remove the dominant cost.
- Do not retry text-piece concat collection as a standalone direct-record
  optimization. Revisit row-build only as a prepare-time literal/parameter
  template with fewer per-row branches, or as part of a bulk page-run path where
  record construction and B-tree layout are fused over many rows.

## 2026-05-06 - Right-edge quick-balance parent-page cache

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially `large_10col`, after the current profile showed
  `btree_insert_ns` around `6.66 ms`, `btree_quick_balance_ns` around
  `2.33 ms` across `2001` attempts, and `2002` conservative reloads.
- Candidate shape: Bε-tree-inspired hot-parent retention in
  `crates/fsqlite-btree/src/balance.rs` and `cursor.rs`: seed a retained
  parent `PageData`/header image on the first same-parent right-edge split and
  reuse it for later quick-balance appends under that parent. Source was
  reverted after measurement.
- Correctness/build proof before measurement: focused btree test coverage
  passed for the new parent-cache no-reread case, existing cached-hint split
  behavior, append writer behavior, and deep rightmost trees; `cargo fmt
  --check -p fsqlite-btree`, `cargo check -p fsqlite-btree --lib`, and
  `cargo clippy -p fsqlite-btree --all-targets -- -D warnings` passed.
- Evidence artifacts:
  - Baseline/post-template-revert profile:
    `tests/artifacts/perf/insert-cpu-profile-crimsongorge-20260506T065505Z/report-record.json`
    and `bench-record.log`.
  - Candidate run:
    `tests/artifacts/perf/right-edge-parent-cache-candidate-crimsongorge-20260506T083531Z/report-record.json`
    and `bench-record.log`.
- Result: rejected and reverted. `large_10col` FSQLite median worsened
  `18.709 ms -> 32.13 ms`, ratio `1.98x -> 3.37x`. The intended
  quick-balance win inverted: `btree_quick_balance_ns` rose
  `2.33 ms -> 5.09 ms`, `btree_insert_ns` rose `6.66 ms -> 10.08 ms`, and
  `commit_roundtrip_ns` rose `2.20 ms -> 9.13 ms`.
- Do not retry parent `PageData` retention as a standalone right-edge split
  optimization. Retaining the mutable parent image adds clone/cache churn that
  dominates the saved parent read. Revisit only inside a true monotonic
  page-run builder that batches parent divider writes instead of updating the
  parent page once per split.

## 2026-05-06 - Detached new-leaf quick-balance split for prepared INSERT

- Target: the same record-size INSERT `large_10col` hot path. This candidate
  followed the parent-cache rejection by trying to remove the fresh right-leaf
  clone/COW behavior instead of retaining the parent.
- Candidate shape: add a detached quick-balance result in
  `crates/fsqlite-btree/src/balance.rs` and route the prepared direct INSERT
  cached-hint path in `crates/fsqlite-core/src/connection.rs` through a mode
  that moves split-created right leaves directly into the transaction write-set
  without retaining a `PageData` image in cursor caches. Source was reverted
  after measurement.
- Correctness/build proof before measurement:
  `cargo fmt --check -p fsqlite-btree -p fsqlite-core`,
  `cargo check -p fsqlite-btree --lib`, `cargo check -p fsqlite-core --lib`,
  focused retained-cache btree tests, and
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
  passed. `cargo clippy -p fsqlite-btree -p fsqlite-core --all-targets -- -D warnings`
  also passed after matching the existing quick-balance result naming lint.
- Evidence artifacts:
  - Baseline/post-template-revert profile:
    `tests/artifacts/perf/insert-cpu-profile-crimsongorge-20260506T065505Z/report-record.json`
    and `bench-record.log`.
  - Candidate run:
    `tests/artifacts/perf/detached-new-leaf-candidate-crimsongorge-20260506T085231Z/report-record.json`
    and `bench-record.log`.
- Result: rejected and reverted. `large_10col` FSQLite median worsened
  `18.709 ms -> 30.31 ms`, ratio `1.98x -> 3.29x`. Quick-balance time again
  worsened (`2.33 ms -> 5.66 ms`), `btree_insert_ns` rose
  `6.66 ms -> 10.70 ms`, and `commit_roundtrip_ns` rose
  `2.20 ms -> 10.13 ms`.
- Do not retry detached split-created leaf ownership as a standalone prepared
  INSERT optimization. The real step-change needs to remove per-split
  quick-balance/commit write-set churn, not reshuffle which layer owns each
  individual split-created page.

## 2026-05-06 - CASS failure-vocabulary refresh while ledger locked

- Scope: last-60-day CASS search for user-requested failure vocabulary. The
  direct exact workspace filter for `/data/projects/frankensqlite` was sparse,
  so the sweep seeded a project session set with `cass search "frankensqlite"
  --days 60 --robot-format sessions --limit 1000` and then searched that set
  plus global `frankensqlite <term>` queries for `rejected`, `reverted`,
  `slower`, `regress`, `abandon`, `worse`, `rollback`, `noise`,
  `didn't help`, `did not help`, `no improvement`, `not worth`,
  `within noise`, `no measurable`, `failed to improve`, and
  `revert it for now`.
- Result: no distinct older benchmark-rejected or correctness-abandoned
  optimization shape was found beyond entries already represented in this
  ledger. Hits reinforced existing no-retry themes: `SqliteValue`
  `Arc<String>` / `Arc<Vec<u8>>` rewrites, broad `SmallVec` churn, direct
  prepared-DML VDBE bypass, stale raw `bench_insert` serializer/VFS/foldhash
  passes, public-row `SmallVec`, and active-transaction/checkpoint false
  leads.
- Excluded as not measured negative attempts: true SHM/mmap correctness
  sessions, UNIQUE/quoting bug fixes, multi-repo commit summaries, and
  `SharedPageLockTable` / `InProcessPageLockTable` scan findings that CASS
  presents as audit/backlog leads rather than tried-and-reverted matrix
  optimizations.
- Future sweeps should keep using a project session set plus targeted
  `cass view` inspection rather than trusting exact workspace matching alone.

## 2026-05-06 - Comprehensive concurrent benchmark runtime reuse

- Target: `comprehensive-bench --quick --filter concurrent`, especially the
  remaining small 2-writer gap where the clean baseline measured
  `2 writers x 1000 rows` at C SQLite `11.760632 ms` versus FrankenSQLite
  `13.767409 ms` (`1.1706x`).
- Candidate shape: in
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs::bench_concurrent_writers`,
  build one `asupersync::runtime::Runtime` per engine/thread-count outside the
  `measure` closure and reuse it across warmup/measurement iterations, after
  `perf record` showed about half of concurrent-filter samples under
  `Runtime::with_config_and_platform` / worker `pthread_create` setup.
  Candidate was tested only in a clean scratch worktree and reverted there;
  shared checkout was not edited.
- Evidence artifacts:
  `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/baseline-concurrent.json`,
  `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/candidate-runtime-reuse-concurrent.json`,
  `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/concurrent-filter.perf.data`,
  `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/perf-flat.txt`,
  and
  `/data/tmp/frankensqlite-purpleotter-concurrent-20260506T0816Z/perf-children.txt`.
- Result: rejected. Runtime reuse reduced absolute times for both engines but
  helped C SQLite much more on the critical low-thread row:
  `2 writers x 1000 rows` worsened from `1.1706x` to `1.7652x`
  (`C 2.438795 ms`, F `4.304958 ms`). `4 writers` also worsened slightly
  from `1.0248x` to `1.0638x`; `8 writers` improved from `0.4531x` to
  `0.3510x` but had very high FSQLite variance (`68.16%` CV), so it does not
  rescue the section.
- Do not retry runtime-pool reuse as a standalone way to close the concurrent
  row in the comprehensive matrix. Revisit only as a deliberate benchmark
  methodology change with same-window full-matrix acceptance criteria, because
  it changes the setup cost envelope and can make the low-thread C SQLite
  comparison harder to beat.

## 2026-05-06 - VDBE SorterRow and cursor-cache broad sweep

- Scope: March CASS history for VDBE hot-path allocation/parsing reductions
  after the user asked for alien/extreme optimization, especially
  `Opcode::SorterData`, pseudo-cursor `Column`, `StorageCursor::payload()`
  reuse, and cursor movement cache invalidation.
- Candidate shape: in `crates/fsqlite-vdbe/src/engine.rs`, add a `SorterRow`
  structure that stores both decoded sort values and the original record blob;
  teach `SorterCursor`/`SorterInsert`/`SorterData` to reuse the cached blob
  instead of re-encoding; add `cached_row` reuse to `StorageCursor`/`MemCursor`;
  and clear that cache across `Next`, `Prev`, `Rewind`, `Seek*`, `Found`,
  `NotFound`, `Insert`, `Delete`, `IdxInsert`, and `IdxDelete` opcode paths.
- CASS evidence: `cass view
  /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json
  -n 222 -C 45` shows the `SorterRow`/cached-blob plan after reverting the
  `SqliteValue` Arc migration; `cass view ... -n 305 -C 75` shows the broad
  cache-clearing sweep; `cass view ... -n 359 -C 25` shows the run discovering
  `engine.rs` had been truncated and attempting to restore it; `cass view ...
  -n 381 -C 12` shows the user had to recover the file and ask whether
  valuable revisions were lost.
- Result: abandoned as unsafe/unusable, not measured. The attempt coupled too
  many opcode movement semantics and cache lifetime rules into one broad edit
  on a very large file, then corrupted/truncated `engine.rs`. It also depended
  on the previously rejected `SqliteValue` Arc idea to contain duplicate
  decoded/blob memory growth.
- Do not retry this as a broad VDBE sweep or full-file rewrite. Revisit only
  as a single narrow opcode/cache lifetime patch with a known-green
  `engine.rs` diff, focused VDBE correctness tests for cursor movement/cache
  invalidation, and a same-window benchmark proving the target query row
  improves outside noise.

## 2026-05-06 - WAL batch metrics atomics

- Target: INSERT commit path, especially `large_10col` 10K rows where profiles
  showed thousands of WAL frames and several milliseconds in commit/WAL append
  phases.
- Candidate shape: in `crates/fsqlite-wal/src/metrics.rs`, add
  `WalMetrics::record_frame_writes(frame_count, bytes_written)` and replace
  the per-frame loop in
  `crates/fsqlite-wal/src/wal.rs::append_finalized_prepared_frame_bytes` with
  one batched pair of relaxed atomic increments after a successful batch
  append. Source was reverted after measurement.
- Correctness proof: `cargo fmt --check` passed, and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-walmetrics-target cargo test -p fsqlite-wal metrics_frame_write -- --nocapture`
  passed the focused metric tests, including the new batch-counting test.
- Evidence artifacts: same-dirty-window A/B under
  `/data/tmp/frankensqlite-purpleotter-walmetrics-ab/`, with reports
  `baseline-insert.json`, `candidate-insert.json`, and comparison
  `compare-insert.json`.
- Result: rejected and reverted. Insert quick weighted score worsened
  `1.5566 -> 1.5979`, average ratio worsened `1.8858x -> 1.9330x`, and
  geomean worsened `1.8180x -> 1.8631x`. Key large rows worsened on absolute
  FSQLite time: single-transaction `large_10col` 10K `28.568 ms -> 29.415 ms`,
  and record-size `large_10col` 10K `30.768 ms -> 31.242 ms`.
- Do not retry WAL metrics batching as a standalone optimization. Revisit only
  if a direct CPU profile shows `GLOBAL_WAL_METRICS.record_frame_write` atomics
  as visible self-time before touching this path.

## 2026-05-06 - Prepared direct INSERT preserialize when MemDB tracking is dirty

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially `large_10col`, where the current profile still showed about
  `5.96 ms` in row construction and `1.86 ms` in serialization.
- Candidate shape: in
  `crates/fsqlite-core/src/connection.rs::execute_prepared_direct_simple_insert`,
  allow the preserialized-record lane when exact MemDatabase row tracking is
  already dirty/unavailable (`!track_memdb_delta`), while preserving the older
  retained COUNT/SUM-cache guard when row tracking is active. Source was
  reverted after measurement.
- Evidence artifacts:
  - Baseline/post-template-revert profile:
    `tests/artifacts/perf/insert-cpu-profile-crimsongorge-20260506T065505Z/report-record.json`
    and `bench-record.log`.
  - Candidate run:
    `tests/artifacts/perf/preserialize-trackfalse-candidate-crimsongorge-20260506T075115Z/report-record.json`
    and `bench-record.log`.
- Correctness smoke before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-preserialize-trackfalse cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`,
  `... test_prepared_count_sum_interest_seeds_cache_on_first_direct_insert -- --nocapture`,
  and `... test_memory_retained_autocommit_count_sum_cache_survives_flush_boundary -- --nocapture`
  all passed; `cargo fmt --check -p fsqlite-core` passed.
- Result: rejected and reverted. The candidate did force
  `serialize_ns` to `0`, but `large_10col` FSQLite median worsened
  `18.709 ms -> 30.00 ms`, with ratio `1.98x -> 3.10x`. The cost moved into
  B-tree/commit work: `btree_insert_ns` `6.66 ms -> 10.01 ms`,
  `btree_quick_balance_ns` `2.33 ms -> 5.03 ms`, and
  `commit_roundtrip_ns` `2.20 ms -> 9.33 ms`.
- Do not retry this gate relaxation as a standalone serializer win. The
  preserialized-record shape changes downstream page/cache behavior enough to
  dominate the saved serialization time. Revisit only with a page-run builder
  that owns the downstream B-tree layout as well.

## 2026-05-06 - Quick-balance staged-parent in-place mutation

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially `large_10col`, after the current profile attributed roughly
  `6.66 ms` to B-tree insert work, `2.33 ms` to quick-balance across `2001`
  attempts and `1995` hits, and `2002` conservative reloads.
- Candidate shape: in
  `crates/fsqlite-btree/src/balance.rs::balance_quick_known_divider_rowid`,
  split parent-divider planning, new-leaf construction, and parent mutation so
  an unpublished staged parent page could be inspected and patched with
  `try_mutate_staged_page_data` instead of being read/cloned/written through
  the normal parent update path. Source was reverted after measurement.
- Evidence artifacts:
  - Baseline/post-template-revert profile:
    `tests/artifacts/perf/insert-cpu-profile-crimsongorge-20260506T065505Z/report-record.json`
    and `bench-record.log`.
  - Candidate run:
    `tests/artifacts/perf/staged-parent-quick-balance-candidate-crimsongorge-20260506T074018Z/report-record.json`
    and `bench-record.log`.
- Correctness smoke before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-staged-parent cargo test -p fsqlite-btree table_append_after_last_position_with_writer -- --nocapture`
  (`2` tests passed),
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-staged-parent cargo test -p fsqlite-btree test_table_insert_prechecked_absent_deep_tree_rightmost_10k -- --nocapture`
  (`1` test passed), and `cargo fmt --check -p fsqlite-btree` passed.
- Result: rejected and reverted. `large_10col` FSQLite median worsened
  `18.709 ms -> 33.27 ms`, with ratio `1.98x -> 3.44x`. The profiled
  subspans also worsened: `btree_insert_ns` `6.66 ms -> 10.79 ms`,
  `btree_quick_balance_ns` `2.33 ms -> 5.36 ms`, and
  `commit_roundtrip_ns` `2.20 ms -> 9.47 ms`. The extra staged-parent
  mutation hook and closure split increased quick-balance cost even on the
  path it was meant to reduce.
- Repeat after the fused direct-record serializer was also rejected. The first
  quick candidate run looked promising (`large_10col` record-size FSQLite
  `17.55 ms`, ratio `1.83x`; single-txn large 10K FSQLite `16.46 ms`, ratio
  `1.67x`), but repeat and isolated record-size runs did not hold. Repeat
  record-size `large_10col` was FSQLite `29.15 ms`, ratio `3.21x`; isolated
  record-size was FSQLite `29.27 ms`, ratio `3.18x`. Artifacts:
  `tests/artifacts/perf/staged-parent-qb-crimsongorge-20260506T101720Z/report-insert-quick.json`,
  `report-insert-quick-repeat.json`, `report-record-only.json`, and
  corresponding `bench*.log` files.
- Do not retry standalone staged-parent parent-divider mutation inside
  `balance_quick_known_divider_rowid`. Revisit only as part of a true monotonic
  page-run builder that eliminates per-split quick-balance calls altogether, or
  with a fresh profile proving the staged parent mutation hook has become a
  zero-cost primitive.

## 2026-05-06 - Quick-balance new-leaf ownership transfer

- Target: `INSERTThroughput - Record Size Comparison (10K rows, single txn)`,
  especially `large_10col`, after the current profile attributed roughly
  `6.66 ms` to B-tree insert work, `2.33 ms` to quick-balance, `2001`
  quick-balance attempts, and `2002` conservative reloads.
- Candidate shape: in
  `crates/fsqlite-btree/src/balance.rs::balance_quick_known_divider_rowid`,
  pass the owned new rightmost leaf `PageData` to `writer.write_page_data` and
  keep the clone only for the returned `QuickBalanceResult`, so the next append
  might mutate the staged page without paying clone-on-write. Source was
  reverted after measurement.
- Evidence artifacts:
  - Baseline/post-template-revert profile:
    `tests/artifacts/perf/insert-cpu-profile-crimsongorge-20260506T065505Z/report-record.json`
    and `bench-record.log`.
  - Candidate run:
    `tests/artifacts/perf/btree-owned-quick-balance-candidate-crimsongorge-20260506T0718Z/report-record.json`.
- Correctness smoke before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-btree-owned cargo test -p fsqlite-btree table_append_after_last_position_with_writer -- --nocapture`
  (`2` tests passed) and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-btree-owned cargo test -p fsqlite-btree test_table_insert_prechecked_absent_deep_tree_rightmost_10k -- --nocapture`
  (`1` test passed).
- Result: rejected and reverted. `large_10col` FSQLite median worsened
  `18.709 ms -> 30.39 ms`, with ratio `1.98x -> 3.27x`. The profiled
  subspans also worsened: `btree_insert_ns` `6.66 ms -> 10.43 ms`,
  `btree_quick_balance_ns` `2.33 ms -> 5.49 ms`, and
  `commit_roundtrip_ns` `2.20 ms -> 8.86 ms`. The retained rightmost-leaf
  hint/cache path is more sensitive to ownership and cache locality than this
  simple clone-order hypothesis assumed.
- Do not retry clone-order or owned-page-transfer tweaks inside
  `balance_quick_known_divider_rowid` as standalone work. Revisit only as part
  of a true bulk page-run builder, or with a fresh profile proving the retained
  append hint no longer depends on the existing ownership split.

## 2026-05-06 - CommitIndex high-water counter sharding

- Target: multi-writer MVCC append/publish throughput, especially the
  `mt-mvcc-bench` 2/4/8-writer rows where
  `CommitIndex::latest_seq()`/global sequence visibility looked like a
  possible contention point.
- Candidate shape: in `crates/fsqlite-mvcc/src/core_types.rs`, replace
  `CommitIndex::latest_global: AtomicU64` with cache-line-sharded high-water
  atomics. Publishers write their shard and `latest_seq()` folds shards to
  recover the global max. Source was reverted after measurement.
- Correctness proof:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-cx0hx-target cargo test -p fsqlite-mvcc test_commit_index_ -- --nocapture`
  passed.
- Evidence artifacts: baseline
  `/data/tmp/frankensqlite-purpleotter-cx0hx-baseline-mt.{json,md}` and
  candidate `/data/tmp/frankensqlite-purpleotter-cx0hx-candidate-mt.{json,md}`.
- Result: rejected and reverted. Baseline ratios were 2 writers `0.50x`,
  4 writers `0.61x`, and 8 writers `2.11x`; candidate ratios were 2 writers
  `0.33x`, 4 writers `0.66x`, and 8 writers `1.78x`. The small 4-writer
  movement did not justify material 2- and 8-writer regressions.
- Do not retry standalone sharding of `CommitIndex::latest_global`. Revisit
  only if a current profile proves high-water global contention dominates a
  target matrix row and the full multi-writer section improves.

## 2026-05-06 - VersionStore publish arena-lock hold split

- Target: MVCC publish/version-chain overhead in
  `crates/fsqlite-mvcc/src/invariants.rs::VersionStore::publish`, after
  looking for lower publish arena-lock hold time.
- Candidate shape: allocate the new version under the arena write lock, drop
  the lock, then perform the chain-head CAS outside that lock, retrying by
  updating `prev` on contention. Source was reverted before matrix measurement.
- Baseline evidence:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-cx0hx-target cargo test -p fsqlite-mvcc bench_publish_visibility_ranges_gate -- --ignored --nocapture`
  produced roughly `2456 ns/publish` with tracking off and `2745 ns/publish`
  with tracking on.
- Result: abandoned before benchmark. The intended lifecycle proof gate,
  `lifecycle::tests::test_publish_write_set_keeps_superseded_version_visible_until_gc`,
  was baseline-red after a full candidate revert (`left: 1`, `right: 2`), so
  the experiment lacked a trustworthy safety gate for version lifetime and GC
  semantics.
- Do not retry this publish lock-split shape until the baseline lifecycle proof
  is isolated/fixed or a different known-green MVCC invariant gate covers
  superseded-version visibility through GC.

## 2026-05-06 - Prepared direct INSERT record-template serializer

- Target: `INSERTThroughput — Record Size Comparison (10K rows, single txn)`,
  especially the `large_10col` row where the profile attributed roughly
  `6.17 ms` to record construction and `1.82 ms` to serialization.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, compile an
  INTEGER/TEXT-only prepared direct-INSERT record template at prepare time,
  evaluate placeholder integer expressions and text-concat lengths at execute
  time, and serialize directly into the shared record scratch buffer before
  falling back to the existing generic prepared serializer for unsupported
  bind values. Source was reverted after measurement.
- Evidence artifacts:
  - Baseline/private-memory journal run:
    `tests/artifacts/perf/private-memory-journal-candidate-crimsongorge-20260506T0514Z/report.json`
    and `stderr.log`.
  - Candidate run:
    `tests/artifacts/perf/prepared-record-template-candidate-crimsongorge-20260506T064002Z/report-record.json`
    and `bench-record.log`.
- Correctness smoke before measurement:
  `cargo fmt --check`,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-template-check cargo check -p fsqlite-core --lib`,
  and focused prepared-direct INSERT tests passed locally.
- Result: rejected and reverted. The candidate barely moved the profiled
  construction work (`row_build_ns` `6.168 ms -> 6.078 ms`,
  `serialize_ns` `1.816 ms -> 1.807 ms`) while the benchmark matrix target
  worsened in the measured run: record-size `large_10col` FSQLite median
  `27.575 ms -> 34.072 ms` and ratio `2.87x -> 3.48x` versus C SQLite. The
  remaining gap is not meaningfully in per-row expression templating.
- Do not retry a standalone prepared record-template serializer. Revisit only
  as part of a true monotonic bulk/page-run builder that removes the repeated
  B-tree quick-balance, page image, and commit-frame costs at the same time.

## 2026-05-06 - Staged-page append `Ok(None)` quick-balance suppression

- Target: INSERT single-transaction and record-size `large_10col` 10K rows
  after the current insert profile showed `4004` B-tree cell assembly calls,
  `2001` quick-balance attempts, and `2002` conservative reloads on the
  `large_10col` 10K rows.
- Candidate shape: in
  `crates/fsqlite-btree/src/cursor.rs::try_append_on_external_rightmost_leaf_hint`,
  remove the quick-balance retry from the staged-page `Ok(None)` branch while
  preserving cell-buffer restoration and overflow-chain cleanup. The source was
  reverted after measurement.
- Evidence artifacts:
  - Baseline insert profile:
    `tests/artifacts/perf/current-insert-profile-icypike-20260506T0325Z/stderr.log`.
  - Candidate insert profile:
    `tests/artifacts/perf/staged-qb-regression-fix-icypike-20260506T0330Z/candidate-stderr.log`.
- Correctness smoke before measurement:
  `cargo fmt --check`,
  `git diff --check`,
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-icypike-staged-qb-fix-tests cargo test -p fsqlite-btree balance_quick -- --nocapture`
  (`5` tests passed remotely), and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-icypike-staged-qb-fix-hint-tests cargo test -p fsqlite-btree table_try_append_cached_rightmost_leaf_hint -- --nocapture`
  (`4` tests passed).
- Result: rejected and reverted. The target counters did not move on the
  `large_10col` 10K rows: `btree_cell_assembly_calls` stayed at `4004` and
  `btree_conservative_reloads` stayed at `2002`. Absolute FrankenSQLite
  medians regressed on the target rows: single-transaction `large_10col` 10K
  worsened `36.33 ms -> 68.22 ms`, and record-size `large_10col` 10K worsened
  `39.08 ms -> 77.59 ms`. The candidate run's lower average ratio
  (`2.19x -> 2.05x`) was from C SQLite host variance, not a FrankenSQLite win.
- Do not retry suppressing only this staged-page quick-balance fallback. Revisit
  the area only with a profile that proves a different branch owns the duplicate
  work and with an interleaved A/B that improves absolute FrankenSQLite medians
  on the current INSERT matrix.

## 2026-05-06 - Broad preserialized direct-INSERT guard relaxation

- Target: direct INSERT row-build/serialization cost after commit `b86786f6`
  added a preserialized-record lane in
  `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: relaxed the committed guard from explicit lazy-MemDB
  transactions to any `:memory:` direct INSERT where materialized row values
  appeared unused after B-tree insertion:
  `!track_memdb_delta || defer_lazy_memdb_materialization`, while still
  excluding FK checks, `REPLACE`, deferred MemDB upserts, and retained
  count/sum cache state. Source was reverted after measurement.
- Evidence artifacts:
  - Baseline/dirty current run:
    `tests/artifacts/perf/current-insert-violetlotus-20260506T033241Z/report.json`.
  - Narrow committed guard run:
    `tests/artifacts/perf/direct-serialize-violetlotus-20260506T035628Z/report.json`.
  - Broad guard candidate:
    `tests/artifacts/perf/direct-serialize-violetlotus-guard2-20260506T040116Z/report.json`.
- Correctness/build proof before measurement:
  `cargo fmt --check`,
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-violetlotus-direct-serialize-target cargo check -p fsqlite-core --lib`
  passed remotely, and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-violetlotus-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
  passed locally for the benchmark binary.
- Result: rejected. The broad guard did activate (`serialize_ns=0` on the
  profiled `large_10col` rows), but it did not produce a clean matrix win:
  average ratio regressed `2.2230x -> 2.2379x`, geomean improved only
  marginally `2.0911x -> 2.0810x`, p90 regressed `3.9923x -> 4.0412x`,
  and p99 regressed `4.0194x -> 4.2779x`. Key absolute FrankenSQLite medians
  were mixed: `large_10col` 1K improved `2.012 ms -> 1.776 ms`, but
  `large_10col` 10K worsened `38.595 ms -> 38.963 ms`, `small_3col` 100
  worsened `0.194 ms -> 0.266 ms`, and `medium_6col` 100 worsened
  `0.275 ms -> 0.368 ms`.
- Do not retry broadly enabling the preserialized direct-INSERT lane merely
  because materialized row values are unused. Revisit only with a narrower
  row-shape/row-count predictor or an interleaved A/B that improves absolute
  FrankenSQLite medians on the target rows without p90/p99 regressions.

## 2026-05-06 - Direct UPDATE lazy decoded-row scratch borrow

- Target: isolated prepared direct UPDATE on the fixed-width REAL fast path
  used by `UPDATE/DELETEThroughput`, after the current profile showed VDBE
  bypass was active and the remaining mutation cost was sub-microsecond
  ceremony plus B-tree page work.
- Candidate shape: in
  `crates/fsqlite-core/src/connection.rs::execute_prepared_direct_simple_update`,
  borrow `prepared_direct_update_row_scratch` only after
  `try_execute_prepared_direct_simple_update_fixed_width_real` declined, so the
  hot fixed-width overwrite lane avoided one per-row RefCell borrow. Source was
  reverted after measurement.
- Evidence commands:
  `hyperfine --warmup 2 --runs 15 "/data/tmp/frankensqlite-baseline-target/release-perf/perf-update-delete 100 20000 update fsqlite isolated" "/data/tmp/frankensqlite-violetlotus-target/release-perf/perf-update-delete 100 20000 update fsqlite isolated"`
  and the same command for `delete`. Baseline was detached HEAD `f3180709`;
  candidate was the local diff on top of `f3180709`.
- Result: rejected. UPDATE improved only within noise:
  baseline `138.7 ms +/- 3.0 ms`, candidate `137.2 ms +/- 3.6 ms`
  (`1.01x +/- 0.03`). DELETE, used as a non-target regression probe, was flat:
  baseline `218.8 ms +/- 5.4 ms`, candidate `219.3 ms +/- 3.7 ms`, with
  baseline nominally `1.00x +/- 0.03` faster.
- Do not retry lazy-borrowing the UPDATE decoded-row scratch as a standalone
  optimization. Revisit only if a profile shows `prepared_direct_update_row_scratch`
  borrowing/clearing as a material fraction of direct UPDATE time and an
  interleaved same-window A/B moves absolute UPDATE medians outside noise.

## 2026-05-06 - CASS strict alias/session-set resweep: broad March bundles are not perf proof

Scope: user-requested CASS expansion of this ledger, restricted to
FrankenSQLite session history from the last 60 days and failure vocabulary such
as `rejected`, `reverted`, `abandoned`, `slower`, `didn't help`,
`did not help`, `regressed`, `rollback`, `no improvement`,
`failed to improve`, `worse`, `within noise`, `no measurable`, and
`keep gate`.

- Search method: built a CASS session set from both explicit repo path aliases,
  because direct `--workspace /data/projects/frankensqlite` remains sparse in
  the stale-but-usable index:
  `cass search "/data/projects/frankensqlite" --days 60 --robot-format sessions --limit 1000 --mode lexical`
  returned `51` sessions, `/dp/frankensqlite` returned `26`, and the combined
  de-duplicated set had `68` sessions. Negative-vocabulary searches then used
  `--sessions-from /tmp/frankensqlite-cass-combined-sessions-violetcove.txt`.
- Useful hit totals inside that strict session set included `rejected` (`39`),
  `reverted` (`29`), `abandoned` (`6`), `slower` (`10`), `didn't help` (`6`),
  `did not help` (`117`), `regressed` (`3`), `rollback` (`137`),
  `no improvement` (`219`), `did not move` (`126`),
  `failed to improve` (`31`), `within noise` (`4`), `no measurable` (`2`),
  and `keep gate` (`5`). The misspelling `abandones` returned `0`.
- High-signal CASS views inspected:
  `/home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json`
  around lines `81` and `138`,
  `/home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-52485ea5.json`
  around line `13`,
  `/home/ubuntu/.claude/projects/-data-projects/026c17f8-4543-415c-9a12-6eb30204a189.jsonl`
  around line `35`, and
  `/home/ubuntu/.claude/projects/-data-projects/45256a1f-8025-445a-8a4c-4f68bc208028.jsonl`
  around line `335`.
- Guardrail: do not treat the March Gemini "extreme optimization" bundle as a
  reusable accepted or rejected perf patch. It mixed hardcoded page-size
  plumbing, `SmallVec` register/program rewrites, hot register helper changes,
  B-tree direct rowid/target-record parsing changes, prepared PK benchmark
  fairness changes, and asupersync/async-VFS planning in one narrative. The
  session history shows repeated stale file views, failed replacements, and
  "engine.rs reverted" confusion, with no same-window matrix proof tying the
  bundle to a durable C SQLite gap closure.
- Guardrail: multi-repo commit-manager CASS hits are not performance evidence.
  The high-ranking `rejected`/`slower` hits around March commit summaries mostly
  describe logical commit grouping, correctness fixes, API renames, rustfmt, or
  ephemeral-file triage. Use them only to locate commits, not to justify a perf
  retry or skip.
- Result: no new distinct artifact-backed performance reject was found beyond
  the existing entries in this ledger. Future agents should only revive an idea
  from these CASS hits after isolating one current code path, proving a current
  profile signal, and running the exact target row against a same-window
  baseline/candidate matrix.

## 2026-05-06 - Direct DELETE tier0 already-staged MVCC marker skip

Scope: `UPDATE/DELETEThroughput`, especially the current worst full-matrix row
`100 rows / delete 5 rows` after `7d6117e1`, where FrankenSQLite measured
`0.425427 ms` vs C SQLite `0.092583 ms` (`4.595x`).

- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs` in a
  clean detached worktree; source was not applied to the shared checkout.
- Candidate shape: add a `Tier0AlreadyStaged` concurrent write tier for
  `SharedTxnPageIo` so writes to an active page that already has a staged-write
  marker skip redundant MVCC marker restaging and write directly into the pager
  transaction.
- Profile evidence: fresh delayed `perf record` on
  `perf-update-delete 100 20000 delete fsqlite isolated` showed
  `TransactionKind::get_page` (`15.90%`),
  `TransactionKind::write_page_data` (`12.49%`),
  `BtCursor<SharedTxnPageIo>::delete` (`10.17%`),
  `__memmove_avx_unaligned_erms` (`6.36%`), and
  `BtCursor<SharedTxnPageIo>::table_seek_for_insert` (`6.05%`).
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next2-proudanchor-20260506T0244Z/summary.md`,
  `candidate-tier0-staged.diff`,
  `delete100-fsqlite-isolated-delay-perf-report.txt`,
  `hyperfine-tier0-staged-isolated-fsqlite.json`, and
  `hyperfine-tier0-staged-standard-fsqlite.json`.
- Result: rejected. Interleaved clean-worktree A/B had baseline faster on both
  probes: isolated delete baseline `227.7 ms +/- 2.6 ms` vs candidate
  `229.7 ms +/- 3.1 ms` (`1.01x` baseline faster), and standard-row proxy
  baseline `29.6 ms +/- 0.6 ms` vs candidate `30.7 ms +/- 0.9 ms` (`1.04x`
  baseline faster).
- Do not retry this exact tier0 already-staged marker skip as a standalone
  UPDATE/DELETE optimization. The repeated marker-stage work is not large
  enough to offset the extra tier classification branch/probe on this workload.

## 2026-05-06 - Direct UPDATE/DELETE reusable SharedTxnPageIo shell

Scope: `UPDATE/DELETEThroughput`, especially the current worst full-matrix row
`100 rows / delete 5 rows` after `7d6117e1`, where FrankenSQLite measured
`0.425427 ms` vs C SQLite `0.092583 ms` (`4.595x`).

- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs` and
  `crates/fsqlite-core/src/connection.rs`; source was reverted after
  measurement.
- Candidate shape: keep a reusable, drained `SharedTxnPageIo` shell on
  `Connection` and refill it for repeated direct-simple UPDATE/DELETE
  executions inside explicit concurrent transactions. This preserved page-level
  MVCC but tried to avoid per-row Rc/RefCell wrapper allocation.
- Profile evidence: delayed `perf record` on
  `perf-update-delete 100 20000 delete fsqlite isolated` showed DELETE-loop
  costs in `TransactionKind::get_page` (`14.59%`),
  `__memmove_avx_unaligned_erms` (`13.48%`),
  `BtCursor<SharedTxnPageIo>::delete` (`11.22%`), `_int_malloc` (`8.42%`),
  `TransactionKind::write_page_data` (`5.62%`), and
  `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` (`4.11%`).
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-next-proudanchor-20260506T0212Z/summary.md`,
  `baseline-delete100-compare-isolated.log`,
  `candidate-delete100-compare-isolated.log`, and
  `delete100-fsqlite-isolated-delay-perf-report.txt`.
- Result: rejected. Same-target A/B moved absolute FrankenSQLite delete time
  from `1580 ns/delete` to `1613 ns/delete` (about `2.1%` slower). The
  C/FrankenSQLite ratio improved only because the C SQLite denominator slowed
  from `293 ns/delete` to `334 ns/delete`.
- Peer review caveat: the pre-candidate `SharedTxnPageIo::into_inner()` path
  used `Rc::try_unwrap` to catch outstanding cursor/storage references. Any
  future reuse attempt must preserve that stray-reference diagnostic before
  draining or stashing the shell.
- Do not retry reusable `SharedTxnPageIo` shell caching for direct UPDATE/DELETE
  as a standalone optimization. Reconsider only if an allocation profile proves
  wrapper allocation dominates and a same-window A/B improves absolute
  FrankenSQLite update/delete medians.

## 2026-05-06 - CASS last-two-month failure-vocabulary addendum

Scope: user-requested CASS resweep restricted to FrankenSQLite history from
the last two months, looking for terms such as `rejected`, `reverted`,
`slower`, `regressed`, `didn't help`, `did not help`, `abandoned`,
`abandones`, `within noise`, `no improvement`, `rollback`, `worse`,
`failed to improve`, `not worth`, `gave up`, `no measurable`, and
`keep gate`.

- Search method: the exact `--workspace /data/projects/frankensqlite` filter
  is too sparse and can include false-positive titles from other repos, so this
  pass used both the explicit path session set and global
  `frankensqlite <term>` searches. The explicit path seed command
  `cass search '/data/projects/frankensqlite' --days 60 --robot-format sessions --limit 1000 --mode lexical`
  returned `51` session paths in the usable-but-stale CASS index. A fresh
  `timeout 120 cass index --json` refresh stayed in `preparing total=0`, so the
  sweep used the existing index plus targeted `cass view` inspection.
- High-signal sessions opened:
  `/home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json`
  around lines `80` and `210`,
  `/home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json`
  around lines `210` and `285`,
  `/home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-84f3c374.json`
  around line `38`, and recent commit-manager summaries under
  `/home/ubuntu/.claude/projects/-data-projects/026c17f8-4543-415c-9a12-6eb30204a189.jsonl`,
  `/home/ubuntu/.claude/projects/-data-projects/16128d2b-9c1f-4615-85ec-babcb706a4a8.jsonl`,
  and
  `/home/ubuntu/.claude/projects/-data-projects/45256a1f-8025-445a-8a4c-4f68bc208028.jsonl`.
- New practical guardrail from the March Gemini sessions: do not repeat a broad
  coupled "optimize everything" patch that mixes VDBE page-size plumbing,
  `SmallVec` register/program rewrites, hot register helper rewrites, B-tree
  seek changes, `SqliteValue` `Arc` conversion, and benchmark fairness changes
  in one pass. Those sessions show repeated partial reverts, stale file views,
  compile/borrow failures, and confusion over which benchmark file was current.
  Split any surviving idea into a narrow patch with one target row, one proof
  test set, and a same-window matrix comparison.
- Confirmed already-recorded no-retry themes: `SqliteValue` `Arc<str>` /
  `Arc<[u8]>` broke serde/cross-crate type constraints; broad `SmallVec`
  sweeps around VDBE ops/registers and `Opcode::MakeRecord` hit borrow and
  dependency failures; prepared-statement benchmark rewrites were benchmark
  fairness fixes, not proof that FrankenSQLite engine rows closed C SQLite
  gaps; and async-VFS / true-asupersync migration remains architecture
  plan-space, not a rejected micro-optimization.
- Recent commit-manager CASS hits were mostly summaries of landed commits,
  correctness work, or ephemeral-file triage. They did not add a new
  artifact-backed performance reject beyond the entries already below.

## 2026-05-06 - CASS chained project-session refresh

Scope: follow-up to the user request to expand this ledger by searching recent
CASS history for failed or abandoned optimization ideas, restricted to sessions
that mention `/data/projects/frankensqlite` since 2026-03-05.

- Search method:
  `cass search "/data/projects/frankensqlite" --since 2026-03-05 --robot-format sessions --limit 0 --mode lexical`
  returned `67` source paths in the current CASS index. The index was usable
  but actively rebuilding (`healthy=false`, `index.rebuilding=true`), so this
  pass did not force another index refresh.
- Negative vocabulary was searched through that session set with
  `cass search <term> --sessions-from /tmp/frankensqlite-cass-project-sessions.txt --since 2026-03-05 --mode lexical`.
  Useful hit totals included `rejected` (`45`), `reverted` (`29`),
  `abandoned` (`9`), `slower` (`10`), `didn't help` (`8`),
  `did not help` (`139`), `regression` (`171`), `rollback` (`149`),
  `no improvement` (`225`), `did not move` (`150`),
  `failed to improve` (`33`), and `revert` (`152`). The misspelling
  `abandones` returned no hits.
- Result: no new distinct artifact-backed performance reject was found beyond
  the many entries already in this ledger. High-signal hits routed back to the
  existing no-retry fences for broad VDBE/page-size/`SmallVec`/`SqliteValue`
  `Arc` sweeps, stale raw `bench_insert` evidence, prepared-DML bypass ideas,
  benchmark fairness changes, direct INSERT/DELETE micro-candidates, and WAL
  publication/checksum experiments.
- Verification note: CASS returned archived `source_path`s that no longer exist
  on disk, so `rg` over those paths is not a reliable follow-up method. Future
  agents should use CASS-native `view`, `expand`, or `export` when inspecting
  this session set.
- Do not treat a sparse direct `--workspace /data/projects/frankensqlite`
  search as proof that the history is empty. Use the explicit path session set
  plus `--sessions-from`, then only record a no-retry item when a hit names a
  concrete candidate and is backed by benchmark artifacts, commit history, or a
  clear correctness-abandonment rationale.

## 2026-05-05 - Recursive CTE direct SUM streaming did not close the gap

Scope: `Subquery & CTE Performance`, specifically
`Recursive CTE (1..1000 SUM)`, after the quick matrix at `c1d2fe19` showed
FrankenSQLite slower than C SQLite on the only remaining subquery/CTE loser.

- Touched during kept candidate:
  `crates/fsqlite-core/src/connection.rs` in commit `1b3b93fc`, followed by
  dead-helper cleanup in `5cee5c6c`.
- Candidate shape: replace the direct recursive CTE SUM consumer's full
  `Vec<Vec<SqliteValue>>` materialization with a streaming evaluator that steps
  the registered `sum` aggregate as each base or recursive frontier row is
  generated, while keeping the existing `UNION` dedup and `INTERSECT`/`EXCEPT`
  fallback behavior.
- Correctness proof:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-reccte-local cargo test -p fsqlite-core recursive_cte -- --nocapture`
  passed (`28` core recursive tests plus filtered conformance recursive CTE
  tests).
- Evidence artifacts:
  `tests/artifacts/perf/current-quick-matrix-20260506T0005Z-proudanchor/report.json`,
  `tests/artifacts/perf/recursive-cte-stream-head-20260506T003108Z-proudanchor/report.json`,
  and
  `tests/artifacts/perf/full-quick-head-20260506T003556Z-proudanchor/report.json`.
- Result: kept for memory/clarity, but not a gap-closing performance win. The
  focused same-head subquery run measured recursive CTE at C SQLite
  `194.9 us` vs FrankenSQLite `227.0 us` (`1.16x` slower), and the full quick
  matrix measured C SQLite `205.5 us` vs FrankenSQLite `254.9 us`
  (`1.24x` slower). Compared with the prior matrix's FrankenSQLite median
  `234.3 us`, the movement is within a noisy envelope and does not remove the
  row from the gap list.
- Do not retry "stream the direct SUM consumer instead of materializing all
  rows" as a standalone recursive CTE optimization. Future work on this row
  should profile the per-iteration direct expression evaluation/frontier loop
  itself, or compare against C SQLite's recursive VM loop, not revisit the
  now-landed materialization removal.

## 2026-05-05 - Quick-balance staged-parent handoff

Scope: `comprehensive-bench --quick --filter insert`, targeting rightmost
B-tree quick-balance during INSERT after commit-phase profiles pointed at page
representation/copy costs.

- Touched during rejected candidate: `crates/fsqlite-btree/src/balance.rs`;
  source was reverted after measurement.
- Candidate shape: have `balance_quick_known_divider_rowid` try
  `PageWriter::try_take_staged_page_data(parent_page_no)` before reading the
  parent page, mutate that parent image directly, and restore it through
  `restore_staged_page_data` instead of writing a second parent image.
- Correctness proof passed in the candidate checkout:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-qb-parent-target cargo test -p fsqlite-btree balance_quick -- --nocapture`
  (`6` tests). The RCH wrapper was interrupted only after tests completed
  because artifact retrieval hung.
- Evidence artifacts:
  `tests/artifacts/perf/qb-staged-parent-proudanchor-20260505T215458Z/baseline-report.json`,
  `baseline2-report.json`, `baseline3-report.json`,
  `candidate-clean1-report.json`, `candidate-clean2-report.json`, and
  `candidate-clean3-report.json`.
- Result: rejected. In the clean same-window A/B, the 3-run median primary
  weighted score improved `1.8298 -> 1.7478`, but the rest of the signal did
  not justify the complexity: average ratio worsened `2.4612x -> 2.4792x`,
  p90 worsened `3.5893x -> 3.7946x`, p99 worsened `3.8592x -> 3.9710x`,
  absolute FSQLite medians regressed on `18/25` rows, and the target
  `large_10col` 10K single-transaction row regressed
  `36.649 ms -> 38.196 ms`.
- Do not retry staged-parent quick-balance handoff as a standalone INSERT
  optimization. Revisit only if a profile shows parent page materialization
  dominating a specific workload and a clean same-window matrix improves the
  large-row absolute medians without p90/p99 regression.

## 2026-05-05 - Prepared direct DELETE scratch-reset narrowing

Scope: isolated prepared direct DELETE after
`dml-mutation-profile-purplecoast-20260505T1830Z` showed DELETE about `5.23x`
slower than C SQLite and still carrying a small
`reset_prepared_direct_insert_statement_scratch` sub-signal.

- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after measurement.
- Candidate shape: skip the broad `PreparedDirectInsertScratchResetGuard` in
  `execute_prepared_direct_simple_delete`, but after a fresh-eyes review add a
  DELETE-specific reset guard in the retained COUNT/SUM cache maintenance path
  so the scratch buffers actually used by DELETE are still cleared on success
  and error.
- Correctness proof for the candidate passed:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-current-target cargo test -p fsqlite-core --lib prepared_delete -- --nocapture`
  (`4` tests). The broader
  `fsqlite-core --test fast_path_separation test_fast_path_prepared_delete`
  failure was reproduced on a clean baseline worktree first and is pre-existing
  (`ud_fast=0`), so it was not attributed to this candidate.
- Evidence artifacts:
  `tests/artifacts/perf/delete-scratch-reset-cyangorge-20260505T1925Z/summary.md`,
  `tests/artifacts/perf/delete-scratch-reset-cyangorge-20260505T1925Z/candidate.diff`,
  and
  `tests/artifacts/perf/delete-scratch-reset-cyangorge-20260505T1925Z/hyperfine-delete-isolated-local-local.json`.
- Result: rejected. The fair local/local A/B on
  `perf-update-delete 10000 1000 delete fsqlite isolated` measured baseline
  `1.3775s +/- 0.0138s` and candidate `1.3712s +/- 0.0126s`, only about
  `0.45%` faster and inside the same-host variance envelope. A first
  local-baseline/RCH-candidate run showed a misleading candidate slowdown and is
  preserved only as a cross-build caution, not a decision signal.
- Do not retry prepared direct DELETE scratch-reset narrowing as a standalone
  optimization. Retry only if a future profile makes statement scratch reset a
  dominant DELETE cost or if a broader measured change removes the retained
  cache scratch dependency entirely.

## 2026-05-05 - Direct DELETE staged-page publication split

Scope: direct DELETE write path after CASS/recent-session follow-up found a
missing rejected PurpleCoast artifact tied to
`dml-mutation-profile-purplecoast-20260505T1830Z`, where isolated DELETE was
about `5.23x` slower than C SQLite.

- Touched during rejected candidate: pager transaction staging internals around
  `SimpleTransaction::get_page`, `StagedPage::published_page`, and
  `StagedPageBacking::Owned`; source was reverted after measurement.
- Candidate shape: when a page already existed in the transaction write set,
  return a transaction-local immutable clone without marking the staged page as
  published, and keep allowing owned staged pages to overwrite after their
  internal immutable snapshot cache had materialized. The theory was that
  repeated direct DELETE reads of the same leaf disabled same-page overwrite
  stealing through the publication marker.
- Correctness proof: focused pager staging tests passed for the candidate.
- Evidence artifacts:
  `tests/artifacts/perf/delete-write-path-purplecoast-20260505T1905Z/summary.md`
  and its `candidate-isolated-compare.log`; baseline comparison came from
  `tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/exact-isolated-compare.log`.
- Result: rejected and reverted. FSQLite isolated total regressed
  `580 ms -> 600 ms`, UPDATE regressed `263 ms -> 273 ms`, DELETE regressed
  `201 ms -> 209 ms`, and DELETE ratio worsened `5.23x -> 5.39x`.
- Do not retry this staged-page publication split unless a fresh profile shows
  a materially different staged-page mechanism, and require an isolated
  update/delete A/B win before any broader matrix run.

## 2026-05-05 - Direct DELETE top-stack clone removal

Scope: direct table-leaf DELETE after CASS/recent-session follow-up found a
missing rejected PurpleCoast clean-worktree artifact from
`/data/tmp/frankensqlite-purplecoast-delete-topclone` at commit `a50dc8ac`.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`;
  source was not kept in the shared checkout.
- Candidate shape: replace the full cloned top `StackEntry` in
  `BtCursor::delete` with copied scalar metadata for leaf-ness, cell count, and
  `separator_repair_for_deleted_leaf_max(top)?`, aiming to avoid a
  PageData/cell-pointer clone before direct table-leaf DELETE.
- Correctness proof passed in the clean worktree:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-delete-topclone-target cargo test -p fsqlite-btree cursor_delete -- --nocapture`
  (`7` tests).
- Evidence artifact:
  `tests/artifacts/perf/delete-top-clone-purplecoast-20260505T1920Z/summary.md`;
  baseline comparison came from
  `tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/exact-isolated-compare.log`.
- Result: rejected. One isolated `both compare` run improved total
  `580 ms -> 566 ms` and UPDATE `263 ms -> 252 ms`, but the targeted DELETE
  row was flat/slightly worse (`201 ms -> 202 ms`, `5.23x -> 5.26x`), and the
  delete-only confirmation regressed `1011 ms -> 1016 ms`.
- Do not retry top-stack clone removal as a standalone DELETE optimization.
  Reconsider only if a future profile shows the clone itself dominating and a
  same-window delete-only confirmation improves absolute FSQLite time.

## 2026-05-05 - External quick-balance retained-hint single authority

Scope: prepared direct INSERT external rightmost-leaf append path in
`crates/fsqlite-btree/src/cursor.rs`, after insert profiles showed large-row
time in B-tree append and quick-balance work.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` in a
  clean candidate worktree; source was not applied to the shared checkout.
- Candidate shape: after `balance_quick_known_divider_rowid` succeeds in
  `try_quick_balance_on_external_rightmost_leaf_hint`, move
  `result.new_page_data` solely into the caller-owned `TableAppendHint` and
  clear the cursor's internal `rightmost_leaf_cache` instead of retaining a
  duplicate `RightmostLeafCacheEntry`.
- Correctness proof for the candidate passed:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-qbcache-candidate-test-target cargo test -p fsqlite-btree table_try_append_cached_rightmost_leaf_hint -- --nocapture`
  (`4` tests). `rch` failed open for the `/data/tmp` worktree, so the command
  ran locally.
- Evidence artifacts:
  `tests/artifacts/perf/external-qb-cache-single-authority-proudanchor-20260505T2118Z/summary.md`,
  `tests/artifacts/perf/external-qb-cache-single-authority-proudanchor-20260505T2118Z/candidate.diff`,
  `tests/artifacts/perf/external-qb-cache-single-authority-proudanchor-20260505T2118Z/baseline-report.json`,
  and
  `tests/artifacts/perf/external-qb-cache-single-authority-proudanchor-20260505T2118Z/candidate-report.json`.
- Result: rejected. The same-host insert quick matrix regressed average ratio
  `2.4990x -> 2.5713x`, geomean `2.3954x -> 2.4847x`, primary weighted score
  `1.7007 -> 1.7335`, write-bulk geomean `2.5568x -> 2.6611x`, and
  write-single geomean `1.4846x -> 1.5027x`. The large 10-column record-size
  ratio looked better (`4.07x -> 3.79x`), but the absolute FrankenSQLite median
  still regressed (`36.54 ms -> 38.74 ms`) while C SQLite moved too.
- Do not retry this single-authority external quick-balance retained-hint
  change as a standalone INSERT optimization. Revisit only if a future profile
  proves the internal post-split cache entry itself is dominant and a same-window
  insert matrix improves absolute FrankenSQLite medians.

## 2026-05-05 - SharedTxnPageIo synthetic page-one cleanup maybe-stale flag

Scope: `comprehensive-bench --quick --filter insert`, after a perf sample showed
`SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` in the direct
INSERT write path and existing tests covered page-one synthetic cleanup
invariants.

- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs`;
  source was reverted after measurement.
- Candidate shape: add a shared `Rc<Cell<bool>>` maybe-stale flag to
  `SharedTxnPageIo`, initialize it from the concurrent handle, set it when
  page-one synthetic conflict tracking is installed, and skip the per-write
  synthetic page-one lock/probe unless the flag is set.
- Correctness proof passed:
  `CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-vdbe shared_txn_page_io -- --nocapture`
  (`15` tests). An earlier RCH run reached the same green test result, but the
  RCH artifact retrieval wrapper was interrupted after tests completed.
- Evidence artifacts:
  `tests/artifacts/perf/synthetic-pageone-clear-candidate-cyangorge-20260505T2120Z/report.json`,
  `tests/artifacts/perf/synthetic-pageone-clear-candidate-cyangorge-20260505T2120Z/stdout.log`,
  `tests/artifacts/perf/synthetic-pageone-clear-candidate-cyangorge-20260505T2120Z/stderr.log`;
  CPU sampling lead:
  `tests/artifacts/perf/insert-cpu-profile-cyangorge-20260505T2111Z/perf.data`
  and
  `tests/artifacts/perf/insert-cpu-profile-cyangorge-20260505T2111Z/report.json`.
- Result: rejected. The quick insert weighted score improved
  `1.7652 -> 1.6954` and geomean moved slightly `2.4209x -> 2.4086x`, but the
  target slow row did not improve: `large_10col` 10K single transaction
  regressed from `38.36 ms` to `39.61 ms`, p99 worsened from `3.95x` to
  `4.58x`, and absolute FSQLite medians were mixed (`13` better, `12` worse).
- Do not retry a synthetic page-one cleanup maybe-stale flag as a standalone
  direct INSERT optimization. Revisit only if a focused profile shows this
  cleanup dominates a target workload and the slow large-row rows improve in the
  same-window matrix.

## 2026-05-05 - CASS synonym sweep coverage note

Scope: user-requested CASS search restricted to FrankenSQLite session history
since `2026-03-05`, using direct `/data/projects/frankensqlite` workspace
filters first and then the archived Gemini workspace alias
`/home/ubuntu/.gemini/tmp/frankensqlite` when the direct workspace filter was
sparse. Searched terms included `rejected`, `reverted`, `slower`,
`regressed`, `didn't help`, `did not help`, `within noise`, `abandoned`,
`abandones`, `no improvement`, `rollback`, `worse`, `failed to improve`,
`no measurable`, `revert it for now`, `not worth`, and `failed the keep`.

- No new benchmark-rejected performance ideas were found beyond the existing
  CASS/artifact sections in this ledger. Useful hits were already represented
  by the Arc/SmallVec, stale raw-benchmark, prepared-DML bypass, async-rewrite
  plan-space, and recent artifact-backed no-retry entries below.
- The remaining hits were intentionally excluded because they were correctness
  fixes that landed, commit-log summaries from multi-repo sessions, accepted
  optimizations, issue-triage text, or CASS false positives where the negative
  word was unrelated to a performance candidate.
- The attempted `cass index --json` refresh timed out after staying in
  `preparing total=0`, so this note is based on the existing CASS index plus
  direct `cass view` inspection of the relevant hits. Refresh CASS before
  repeating this sweep only if newer sessions need to be included.

## 2026-05-05 - Exact-path CASS session-set follow-up

Scope: follow-up to the user request to restrict CASS mining to this project
folder and the last two months. Because direct
`--workspace /data/projects/frankensqlite` was sparse and returned at least one
cross-project false positive, the search first built a session set from CASS
sessions that explicitly mention `/data/projects/frankensqlite`, then searched
only those sessions with `--sessions-from` and `--days 60`.

- Session seed command:
  `cass search '/data/projects/frankensqlite' --days 60 --robot-format sessions --limit 500 --mode lexical`
  returned `38` session paths in the existing CASS index.
- Negative terms searched inside that seed set included `rejected`,
  `reverted`, `slower`, `regressed`, `didn't help`, `did not help`,
  `abandoned`, `abandones`, `within noise`, `no improvement`, `rollback`,
  `worse`, and `failed to improve`, plus benchmark/perf phrase combinations.
- No additional benchmark-rejected performance candidates were found that were
  not already represented elsewhere in this ledger. The high-signal perf hits
  led back to existing entries such as stale March hash/cache experiments,
  page-1 synthetic hint state, WAL/checksum/publication candidates, direct
  INSERT row-build candidates, and benchmark-policy rejects.
- Excluded hits were non-perf or non-negative: multi-repo commit grouping
  summaries, FrankenTUI accessibility sessions indexed under a broad workspace,
  SHM correctness work with pre-existing harness failures, UNIQUE/quoting bug
  fixes, and landed feature summaries.
- Practical rule for future sweeps: prefer this explicit-path session-set
  method over trusting the exact workspace filter alone, then add only
  artifact-backed perf rejects or correctness-abandoned optimization attempts.

## 2026-05-05 - CASS user-term dedupe refresh

Scope: follow-up to the explicit request to search last-two-month project history
for failure vocabulary such as `rejected`, `reverted`, `slower`,
`didn't help`, `did not help`, `abandoned`, `abandones`, `within noise`,
`no improvement`, `rollback`, `worse`, `failed to improve`, and `not worth`.
The existing CASS index was stale but usable. A direct session seed for
`/data/projects/frankensqlite` returned `38` session paths; direct
`--sessions-from` searches reported term totals without usable snippets, so the
fallback was global `frankensqlite <term>` CASS search plus targeted `cass view`
inspection of only source paths/titles clearly tied to this repo.

- No additional benchmark-rejected or correctness-abandoned optimization
  candidates were found beyond entries already represented in this ledger.
- Hits for the March `bench_insert` serializer/VFS/hash-map optimization pass
  reinforce the existing stale-benchmark rule: the raw benchmark moved only
  about `0.271 s` to `0.265 s` while thrashing parse/codegen with unique SQL
  strings. Do not use that run as a keep/retry signal for current insert work.
- Hits for `SqliteValue` `Arc<str>` / `Arc<[u8]>`, prepared-DML direct VDBE
  execution, and public `Row` `SmallVec` were already covered by the CASS
  last-60-day no-retry expansion below.
- Hits for async VFS / true-asupersync migration beads were already classified
  as architecture plan-space, not a rejected micro-optimization.
- Hits for `ConcurrentRegistry` global-lock stripping, VDBE/B-tree index-record
  parse hoists, cancellation checkpoints, and JSON/VFS correctness audits were
  excluded because CASS presented them as accepted or correctness-focused work,
  not as ideas that were tried and abandoned. Add them later only if a commit,
  artifact, or follow-up session shows a measured revert or keep-gate failure.
- A repeat phrase pass also searched `gave up`, `abandon*`, `no measurable`,
  and `keep gate`. The only new-looking CASS leads routed back to already
  recorded March hardcoded-page-size, broad `SmallVec`, and prepared-benchmark
  fairness work; none added a new benchmark-rejected performance shape. Do not
  use `rg` over CASS `source_path`s as the primary verification method here:
  several indexed session paths are archived/virtual and no longer exist on
  disk, while `cass view <source_path> -n <line> -C <context>` still resolves
  them from the CASS index.

CASS evidence inspected in this refresh:
- `cass search '/data/projects/frankensqlite' --days 60 --robot-format sessions --limit 500 --mode lexical`
- `cass search 'frankensqlite rejected' --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite reverted' --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite slower' --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite abandoned' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search "frankensqlite didn't help" --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite did not help' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite within noise' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite no improvement' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite rollback' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite worse' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite failed to improve' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite not worth' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite gave up' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite abandon*' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite no measurable' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite keep gate' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 104 -C 60`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 285 -C 24`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-f0efb944.json -n 219 -C 28`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-52485ea5.json -n 13 -C 24`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-466c7bcd.json -n 168 -C 30`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T00-01-e13b2d1e.json -n 4 -C 20`

## 2026-05-05 - Direct DML `SharedTxnPageIo` wrapper reuse

Scope: prepared direct INSERT/UPDATE/DELETE in concurrent mode, after the
UPDATE/DELETE profile showed fixed setup costs around short-lived B-tree cursor
and page I/O wrapper construction.

- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-vdbe/src/engine.rs`; source was reverted after measurement.
- Candidate shape: park a reusable `SharedTxnPageIo` wrapper on `Connection`,
  refill it with the current pager transaction plus concurrent writer context
  for each direct DML statement, then drain the transaction back to
  `active_txn`. The intent was to avoid rebuilding the internal
  `Rc<RefCell<...>>` pair for every prepared direct INSERT/UPDATE/DELETE row.
- Correctness smoke for the candidate passed:
  `cargo fmt --check` and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-dml-pageio-target cargo test -p fsqlite-vdbe shared_txn_page_io --profile release-perf -- --nocapture`
  (`15` matching tests). A broader `fsqlite-core` filtered test attempt was
  killed after the remote command ran silently for more than ten minutes, so
  the keep/revert decision used benchmark evidence instead.
- Evidence artifacts:
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/baseline-update-report.json`,
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/update-report.json`,
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/baseline-insert-report.json`,
  and
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/candidate-insert-report.json`.
- Result: rejected. Same-machine A/B showed the INSERT FrankenSQLite median
  geomean improved only `0.9%` while the C-relative geomean regressed `2.2%`
  (`25` scenarios, `14` FSQLite medians up and `11` down). UPDATE/DELETE was
  effectively flat on FSQLite geomean (`0.36%` slower), regressed the tiny
  delete row by `21.7%`, and regressed the C-relative geomean by `13.9%`.
- Do not retry direct DML `SharedTxnPageIo` wrapper reuse as a standalone
  optimization. The allocation avoided here is too small and too noisy relative
  to row-build, B-tree, pager, WAL, and benchmark fixed costs.

## 2026-05-05 - Stage-only external quick-balance retained hint

Scope: prepared direct INSERT rightmost-leaf append path, after profiles showed
large-row time in B-tree quick-balance and `PageData` clone/retention around
`try_quick_balance_on_external_rightmost_leaf_hint`.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`;
  source was reverted after measurement.
- Candidate shape: after `balance_quick_known_divider_rowid`, skip retaining
  the new leaf `PageData` in the caller-owned external `TableAppendHint` when
  the pager can mutate staged `PageData` directly. The measured version also
  preserved the old retained-page behavior for non-staged PageWriters and added
  a staged-page quick-balance fallback when the staged hinted leaf fills.
- Correctness note: the first stage-only attempt was rejected before
  benchmarking because
  `test_table_try_append_cached_rightmost_leaf_hint_reuses_retained_leaf_image`
  found row-order corruption (`59` expected, `95` observed). The measured
  staged-capability guarded candidate passed the focused clean-worktree proofs:
  `cargo fmt --check`,
  `cargo test -p fsqlite-btree table_try_append_cached_rightmost_leaf_hint --profile release-perf -- --nocapture`
  (`4` matching tests), and
  `cargo test -p fsqlite-core prepared_direct_simple_insert_implicit_rowid --profile release-perf -- --nocapture`
  (`3` matching tests). Shared worktree verification was blocked at the time by
  an unrelated dirty `crates/fsqlite-pager/src/pager.rs` compile error, so the
  proof and benchmark used a clean detached worktree at `f7ea3cdd`.
- Evidence artifacts:
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/baseline-insert-report.json`,
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/candidate-insert-report.json`,
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/ab-summary.json`,
  and
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/summary.md`.
- Result: rejected. Same-window insert quick matrix had `10` FSQLite median
  wins and `15` regressions, with FSQLite geomean `1.0254x`
  candidate/baseline (`2.54%` slower). C-relative ratio geomean improved to
  `0.9590x`, but this was driven by C-side timing movement rather than absolute
  FSQLite improvement. The target `large_10col` 10K single-txn row improved
  `37.483 ms -> 36.182 ms`, but record-size `large_10col` 10K regressed
  `35.613 ms -> 36.716 ms`; small/medium rows regressed materially, including
  `small_3col` 1000 `+18.0%` and small transaction-strategy 10K single txn
  `+11.3%`.
- Do not retry this stage-only retained-hint clone avoidance as a standalone
  B-tree optimization. The retained leaf image is a useful fallback/rollback
  shape, and removing it does not improve the end-to-end insert matrix even
  when correctness is preserved for staged-capable writers.

## 2026-05-05 - Large borrowed WAL commit threshold

Scope: `comprehensive-bench --quick --filter insert`, targeting the large-row
commit path after `insert-commit-profile-cyangorge-20260505T1615Z` showed
`pager::build_group_commit_batch` cloning staged pages into owned
`TransactionFrameBatch` frames.

- Touched during reverted candidate: `crates/fsqlite-pager/src/pager.rs`.
- Candidate shape: promote the borrowed `collect_wal_commit_batch` helper out
  of test-only code and, for commits with at least `512` frames, bypass the
  owned group-commit batch by appending borrowed frame refs directly while
  still checking the pinned WAL conflict snapshot, using prepared-frame
  validation, taking the DB-file `Reserved` lock, honoring sync policy, and
  updating `inner.db_size`.
- Correctness checks: `cargo test -p fsqlite-pager test_collect_wal_commit_batch
  -- --nocapture` passed (`4` tests), and `cargo test -p fsqlite-pager
  group_commit -- --nocapture --test-threads=1` passed (`22` tests). The same
  `group_commit` filter without serialized test execution showed existing
  fault-hook interference between tests, so the serialized rerun was used for
  the candidate check.
- Evidence artifact:
  `tests/artifacts/perf/group-commit-large-borrowed-cyangorge-20260505T1650Z/summary.md`.
- Result: abandoned/reverted. The benchmark run was contaminated by an
  unrelated dirty `crates/fsqlite-btree/src/cursor.rs` diff that appeared while
  measuring, but the candidate was not promising enough to justify an isolated
  repeat: weighted insert score worsened `1.699053 -> 1.787694`, geomean ratio
  worsened `2.362302x -> 2.390798x`, `write_bulk` worsened `2.515348x ->
  2.526914x`, and `write_single` worsened `1.490767x -> 1.592921x`. Target
  FSQLite medians did not improve cleanly (`large_10col` 10K
  `36.165071 ms -> 37.493052 ms`, record-size large 10K
  `37.055950 ms -> 37.160930 ms`, record-size medium 10K
  `9.888943 ms -> 11.164965 ms`).
- Do not retry this exact borrowed large-commit threshold without an isolated
  A/B and a proof that bypassing the queue still preserves the group-commit
  fault/publish semantics under concurrent writers.

## 2026-05-04 - CASS archaeology guardrails

Scope: `cass` searches restricted to FrankenSQLite content since `2026-03-04`,
covering terms such as `rejected`, `reverted`, `abandoned`, `slower`,
`regressed`, `did not help`, `no improvement`, `within noise`, `rollback`,
`candidate`, `benchmark`, and `matrix`.

- `SqliteValue` `Arc` wrapping (`Arc<str>`, `Arc<[u8]>`, `Arc<String>`,
  `Arc<Vec<u8>>`) showed up repeatedly as a clone-reduction idea, but March
  fresh-eyes sessions report that the attempt broke serde/type constraints and
  left cross-crate type mismatches. Do not retry without a designed serde story
  and a compile/test proof before measuring.
- Broad `SmallVec` register/op sweeps caused dependency, initialization, and
  borrow-check failures around `VdbeProgram`, `VdbeEngine::registers`, and
  `Opcode::MakeRecord`; the safe recovery was to restore owned clones before
  mutably borrowing storage cursors. Do not repeat as a broad mechanical sweep.
- A broad "alien" batch combining multi-tiered SSI witness indexing, B-tree
  stack elision, Adaptive Sharded ARC, and CAMP produced correctness hazards:
  custom/global witnesses were dropped, dirty write-set pages could be hidden by
  stack elision, `ArcCache::get` could deep-clone page data, witness bridge
  methods were lost during edits, and the CAMP path initially used `unsafe`.
  Revisit only as narrow, separately measured patches with SSI/witness and
  dirty-page correctness tests.
- `with_pager_write_txn` bypassing active transactions was a CASS false lead:
  the same session re-read the helper and corrected itself that the function is
  centralized and handles active transactions. Do not spend another pass on that
  theory without new evidence.
- Audit-only CASS leads such as `OP_Count` full-table scans, `cursor_column`
  payload comparison cost, parse-cache full flushes, index-ordered OFFSET after
  column reads, and Bloom one-hash false positives should remain optimization
  backlog, not negative results, until someone has a measured rejected patch.

Primary CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-84f3c374.json -n 44 -C 6`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-5b9da3d6.json -n 153 -C 18`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 267 -C 10`

## 2026-05-05 - Additional CASS-derived rejected candidates

Scope: last-two-month FrankenSQLite session history searched for negative
signals such as `rejected`, `reverted`, `slower`, `regressed`, `didn't help`,
`did not help`, `within noise`, `abandoned`, and nearby misspellings.

- `concurrent_page_state` structural rewrite / empty-map short-circuit:
  rejected after micro results only moved `1.6 ns` to `1.5 ns` on the empty
  case while populated lookup barely moved (`+0.1%`); the patch was reverted.
  Do not retry without a real matrix row showing state lookup dominates.
- WAL checksum transform hand-folding: rejected after the hand-folded checksum
  path measured roughly `30%` slower than the existing implementation. Do not
  retry scalar checksum reshuffling unless a CPU profile isolates checksum math
  and the candidate is checked against WAL benchmark rows.
- PAX-style `Column` decode cache: deprioritized because the important decode
  cache had already landed and later traces showed different hotspots. Do not
  reopen this as a generic "cache decoded column" idea without proving the
  current row shape is missing the existing cache.
- Same-page `PageBuf` steal allocator: a proof test passed, but wall-clock
  movement was within noise. Do not retry as allocator surgery unless a fresh
  profile shows page-buffer allocation, not pager/VDBE work, dominates.
- Statement-renewal micro-batcher: abandoned after small-N benchmark movement
  stayed within noise; a naive deadline check using `Instant::now()` regressed.
  Do not retry per-call time checks in the hot path.
- `PageData` `Arc<Vec<u8>>` to `Arc<[u8]>`: deferred as high-risk and low
  isolated expected value. Do not attempt as a broad type rewrite without a
  migration plan covering all pager/WAL/MVCC consumers and a matrix target.
- Rust PGO plus full LTO for INSERT: rejected after INSERT benchmarking showed
  roughly `20-25%` slower results. Do not repeat toolchain/profile flag
  exploration for insert throughput unless the profile setup itself changes.

## 2026-05-05 - Quick-balance one-cell pointer Vec pooling

Scope: insert-only comprehensive e2e matrix after
`199bd14b perf(btree/balance): gate balance_quick on the exact divider size`,
targeting the quick-balance success path in
`crates/fsqlite-btree/src/cursor.rs`.

- Candidate shape: add a helper that takes a `Vec<u16>` from the existing
  thread-local cell-pointer pool and pushes the single new-cell pointer,
  replacing the two `vec![result.new_cell_ptr]` allocations used after
  `balance_quick_known_divider_rowid` succeeds.
- Behavior proof: `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target
  cargo test -p fsqlite-btree rightmost_leaf_hint -- --nocapture` passed
  (8 tests).
- Evidence: baseline artifact
  `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/report.json`;
  candidate artifact
  `tests/artifacts/perf/insert-quick-balance-pointer-pool-cyangorge-20260505T120405Z/report.json`
  and `run.log`.
- Result: rejected and reverted. The summary ratios looked better, but they
  were distorted by C SQLite variance. Engine-side medians regressed on the
  split-heavy single-transaction rows (`large_10col` 10K `34.756 ms` ->
  `37.287 ms`, 100K `415.902 ms` -> `451.660 ms`) and the hot counter moved
  the wrong way (`btree_quick_balance_ns` for `large_10col` 10K `4.309 ms` ->
  `5.262 ms`). Do not retry the one-cell pooled-Vec helper unless allocator
  profiling proves those tiny `Vec` allocations dominate and the thread-local
  pool access can be made cheaper than allocation.

## 2026-05-05 - Direct UPDATE fixed-width REAL one-byte header offset

Scope: `perf-update-delete 10000 40 update`, targeting the prepared
`UPDATE bench SET value = ?2 WHERE id = ?1` direct-simple fixed-width REAL
path in `crates/fsqlite-core/src/connection.rs`.

- Candidate shape: after `BtCursor::payload_into`, bypass
  `parse_record_projected_column_offsets` for records whose header is exactly a
  one-byte header-size varint plus one-byte serial types, validate the target
  serial type is REAL (`7`), compute the column payload offset by summing the
  preceding one-byte serial lengths, and fall back to the generic parser for
  every other record shape.
- Behavior proof: added a direct helper test comparing the computed offset to
  the generic projected-column parser, plus the existing direct-simple REAL
  update proof still passed under `rch exec -- env
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-connection-target cargo
  test -p fsqlite-core real_column -- --nocapture` (2 matching tests passed).
- Evidence: paired release-perf hyperfine artifact
  `tests/artifacts/perf/direct-update-real-offset-candidate-cyangorge-20260505T0838Z/hyperfine-update.json`.
- Result: rejected and reverted. Baseline averaged `344.2 ms +/- 6.9 ms`;
  candidate averaged `347.2 ms +/- 5.4 ms`, so the unpatched binary was
  `1.01x +/- 0.03` faster. Do not retry header-offset microparsing for this
  direct UPDATE path unless a fresh profile shows projected record-header parse
  dominating wall time rather than page write, payload copy, or insert setup.

## 2026-05-05 - Direct UPDATE fixed-width REAL payload-range patch

Scope: `perf-update-delete 10000 40 update`, targeting the prepared
`UPDATE bench SET value = ?2 WHERE id = ?1` direct-simple fixed-width REAL
path after the one-byte header-offset candidate still left full-payload copy and
same-size overwrite work in the hot path.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: add a B-tree helper that borrows the current local
  no-overflow table payload for record-header inspection, plus a second helper
  that patches only the 8-byte REAL value range in the current leaf payload.
  The direct UPDATE path used these helpers to avoid `BtCursor::payload_into`
  and avoid copying the whole payload back through
  `table_overwrite_current_payload_same_size_no_overflow`.
- Behavior proof: focused B-tree helper test passed, and
  `test_direct_simple_update_single_real_column_patches_payload_without_decode`
  passed after adding an assertion that the fixed-width REAL path performs zero
  local-payload copy calls.
- Evidence: paired release-perf hyperfine artifact
  `tests/artifacts/perf/direct-update-real-range-patch-candidate-cyangorge-20260505T0900Z/hyperfine-update.json`.
- Result: rejected and reverted. Baseline averaged `348.6 ms +/- 5.7 ms`;
  candidate averaged `354.1 ms +/- 8.2 ms`, so the unpatched binary was
  `1.02x +/- 0.03` faster. Do not retry this two-helper payload-range patch as
  an UPDATE microcopy optimization unless a fresh profile proves payload copy is
  again dominant and the B-tree helper overhead has been removed or amortized.

## 2026-05-05 - Direct UPDATE fixed-width REAL projected-byte page patch retry

Scope: fresh retry of the fixed-width REAL direct UPDATE payload-range idea
after a current isolated `perf-update-delete 10000 1200 update` CPU profile
again showed time in `memmove`, `read_cell_pointers_into`,
`parse_record_projected_column_offsets`, `write_page_data`, and
`table_overwrite_current_payload_same_size_no_overflow`.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`; source was reverted after the
  benchmark section failed the keep bar.
- Candidate shape: add one B-tree helper to parse the current no-overflow table
  payload directly from the leaf page and copy only the projected replacement
  bytes for the updated REAL column, then run the direct fixed-width REAL UPDATE
  fast path before borrowing generic row/payload scratch buffers.
- Focused proofs passed:
  `env CARGO_TARGET_DIR=.rch-target cargo check -p fsqlite-core -p fsqlite-btree`,
  `env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree test_table_patch_current_payload_projected_bytes_no_overflow_updates_column_only -- --nocapture`,
  `env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree table_overwrite_current_payload_same_size_no_overflow -- --nocapture`,
  and `env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-core direct_simple_update -- --nocapture`.
- Evidence artifacts:
  `tests/artifacts/perf/update-payload-range-proudanchor-20260505T2340Z/summary.md`,
  `candidate-update-section-report.json`, and
  `baseline-update-section-report.json`.
- Result: rejected. The narrow isolated harness improved by about `5%`
  (`880/886 ns` baseline per update in same-window reverse builds versus
  `838/839 ns` candidate), but the quick `UPDATE/DELETEThroughput` section was
  mixed: 10K update improved `10.34 ms -> 9.70 ms` and 10K delete improved
  `9.21 ms -> 8.63 ms`, while 100-row update regressed
  `451.7 us -> 468.5 us`, 1000-row update regressed `1.26 ms -> 1.32 ms`, and
  1000-row delete regressed `1.19 ms -> 1.22 ms`. FSQLite geomean for the
  section moved only `0.993x` candidate/base, below the keep threshold.
- Do not retry direct UPDATE projected-byte page patching as a standalone
  microcopy optimization. Reconsider only with a design that improves all
  UPDATE rows or produces a section-level FSQLite geomean win large enough to
  overcome the small/mid-row regressions.

## 2026-05-05 - Additional CASS/artifact-backed rejects to avoid repeating

Scope: follow-up sweep of the last-two-month CASS hits, recent commits, and
artifact result files for ideas that were measured, rolled back, or explicitly
kept out of the tree but did not yet have a ledger entry.

- `MemDatabase` row-value `Arc<[SqliteValue]>` container swap: rolled back
  after the target `perf-update-delete 10000 10 both` run regressed from
  `264.6 ms +/- 3.9 ms` to `271.5 ms +/- 4.5 ms`, despite passing
  `rch exec -- cargo check -p fsqlite-vdbe -p fsqlite-core --all-targets`.
  Evidence: `docs/perf-a1-memdb-row-values-conclusion.md` and commit
  `0319ea00`. Do not retry shared row-value ownership without an independent
  snapshot-design reason; the narrower `parse_record_into` destination-slot
  idea is the only documented fallback, and only if the clone band grows above
  the ship threshold.
- Direct INSERT rowid-alias borrow: rejected after a behavior proof passed but
  alternating A/B runs on `perf-update-delete 10000 50 both` moved median total
  from `858 ms` to `872 ms` and populate from `412 ms` to `418 ms`. Evidence:
  `tests/artifacts/perf/20260427T1700Z-azurepine-direct-insert-rowid/RESULT.md`.
  Do not retry rowid-alias borrowing as the direct INSERT lever.
- Direct INSERT stateless append hint: rejected after both isolated and
  current-HEAD comparisons made populate slower by roughly `1-2%`. Evidence:
  `tests/artifacts/perf/20260427T2005Z-azurepine-direct-insert-stateless-hint/RESULT.md`.
  Do not retry by dropping retained append-hint page images from explicit
  transactions unless the B-tree hint contract changes materially.
- Synthetic page-one hint cache: rejected after `perf-update-delete 10000 100
  both` median regressed by `5.04%` (`1.2366 s` to `1.2990 s`). Evidence:
  `tests/artifacts/perf/20260428T034415Z-sapphirecrane-next-profile/RESULT-clear-hint-rejected.md`
  and commit `f113fe8c`. Keep the predicate-only stale synthetic page-one
  helper unless a profile proves page-one cleanup dominates a current workload.
- Prepared direct INSERT expression fast path: rejected after targeted concat
  and `?N op literal` handling made the same DML workload mean `3.55%` slower
  while median stayed noise-level. Evidence:
  `tests/artifacts/perf/20260428T1908Z-sapphirecrane-expr-fast/RESULT-expr-fast-rejected.md`.
  Do not add expression-shape special cases without an insert-section A/B win.
- Direct leaf payload writer for prepared INSERT: rejected after the writer
  callback/exact-size route regressed mean by `2.27%` and median by `1.07%`.
  Evidence:
  `tests/artifacts/perf/20260428T1925Z-sapphirecrane-direct-page/RESULT-direct-page-rejected.md`
  and commit `0743bc17`. This is distinct from the later retained-leaf writer
  append entry below; both measured the same basic idea as a loss.
- Direct DML cursor scratch reuse: rejected after interleaved hyperfine showed
  clean parent `1.262 s` versus scratch-routing patch `1.270 s`. Evidence:
  `tests/artifacts/perf/20260428T2135Z-sapphirecrane-direct-dml-cursor-scratch/RESULT-direct-dml-cursor-scratch.md`
  and commit `80777b6b`. Do not retry cursor scratch swaps without a broader
  cursor-owned mutation scratch API and an update/delete-isolated benchmark.
- Direct-simple UPDATE/DELETE schema-proof microbatch carry: committed as
  `4b8151fc` and forward-reverted by `df032429` after measured DML rows and
  the narrow update/delete profiler regressed. Do not reapply schema-proof carry
  to direct UPDATE/DELETE unless the validation cost is proven to dominate and
  the exact DML matrix rows improve.
- Unguarded grouped join aggregate indexed-cache carry: rejected because it
  improved only the 100-row grouped case while dense joins regressed badly
  (`JOIN + GROUP BY` 10K `11.8966 ms` to `14.1428 ms`; `JOIN + HAVING` 10K
  `10.6338 ms` to `15.4707 ms`). Evidence:
  `tests/artifacts/perf/join-grouped-index-cache-candidate-purplecoast-20260504T2040Z/summary.md`.
  Keep the guarded path shape; do not remove the density/table-size guard based
  on small-row wins alone.

## 2026-05-05 - CASS follow-up: stale targets and older no-retry artifacts

Scope: second CASS sweep restricted to FrankenSQLite last-two-month history,
using negative-result terms such as `rejected`, `reverted`, `slower`,
`regressed`, `abandon*`, `did not help`, `within noise`, `worse`, and
`rollback`, then cross-checking matching repo artifacts before adding entries.

- Pre-prepared-statement benchmark ratios are stale routing evidence, not
  current engine targets. March CASS records show a large artificial penalty
  where FrankenSQLite benchmark loops used dynamic `execute(format!(...))`
  while the C SQLite side used prepared statements; commit
  `473f82c3 perf(e2e): convert benchmarks to prepared statements for
  structurally fair comparisons` fixed that class. Do not reuse the old
  `read_count_star 275x` / read-heavy ratios as current target selection
  without rerunning the current benchmark matrix. Do not count benchmark-harness
  rewrites as engine wins unless the asymmetry still exists in current code.
- Tiny ASCII `lower()` / `upper()` stack-buffering in
  `crates/fsqlite-func/src/builtins.rs` was rejected after the string-function
  row failed to show a clean end-to-end win. Evidence:
  `tests/artifacts/perf/string-small-ascii-case-purplecoast-20260504T1940Z/summary.md`.
  Do not retry this exact tiny-ASCII case-conversion lever without a cleaner
  A/B harness and all affected string-function rows improving.
- JSON path array-index ASCII parsing in
  `crates/fsqlite-ext-json/src/lib.rs::resolve_path` was rejected. Forward
  A/B favored baseline (`711.238 ms` vs `731.814 ms`), reverse A/B favored the
  candidate only noisily (`726.703 ms` vs `717.422 ms`). Evidence:
  `tests/artifacts/perf/20260428T1845Z-icybluff-json-path-index/RESULT.md`.
  Do not retry local digit parser specialization for JSON paths unless a
  process-level benchmark clears the stability bar.
- WAL frame assembly v2, which built a local 24-byte frame header and appended
  header plus payload instead of the committed field-by-field helper, was
  rejected because current-head v1 was slightly faster (`327.444 ms` vs
  `330.427 ms`). Evidence:
  `tests/artifacts/perf/20260428T0920Z-icybluff-wal-frame-assembly/RESULT.md`.
  Keep the existing `push_wal_frame_bytes` shape unless a fresh WAL benchmark
  shows a real frame-assembly hotspot.
- WAL checksum `then_aligned_bytes` streaming was rejected as within noise:
  candidate `329.915 ms` versus baseline `331.209 ms`, a `0.39%` delta inside
  run sigma. Evidence:
  `tests/artifacts/perf/20260428T0900Z-icybluff-wal-checksum/RESULT.md`.
  Do not retry checksum-transform reshaping based on sub-1% microbench movement.
- B-tree delete sort-record narrowing was rejected. Replacing
  `(usize, usize, usize)` triples with a compact `CellMove` did not improve the
  target path; longest check was flat/slower overall (`7885 ms` to `7902 ms`)
  while delete regressed by about `11.3%`. Evidence:
  `tests/artifacts/perf/20260427T1855Z-azurepine-btree-sort-record/RESULT.md`.
  Do not retry by shrinking the carried sort record width alone.
- Compact table-leaf delete sub-ideas: deferred scratch reuse and unrefined
  physical-neighbor delete were both rejected while the refined accepted path
  was kept. Deferred scratch reuse showed no measured win, and applying the
  physical-neighbor path to all compact leaves regressed delete-only. Evidence:
  `tests/artifacts/perf/20260427T2348Z-snowyfortress-next-hotspot/RESULT.md`.
  Do not replace the cheaper descending fast path or reintroduce scratch reuse
  without a delete-only win.
- Profile-pass hypotheses rejected as primary causes: syscall I/O and
  lock/futex contention were explicitly ruled out as first targets. Evidence:
  `tests/artifacts/perf/20260424T212631Z-profile-pass/HYPOTHESIS_LEDGER.md`
  and `tests/artifacts/perf/20260424T212631Z-profile-pass/REPORT.md`.
  For mixed/insert OLTP, start from row materialization, decode, cursor
  traversal, commit maintenance, memdb reload, and snapshot cloning before
  spending another pass on syscall or futex tuning.

Primary CASS evidence for the stale-target and false-lead guardrails:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-84f3c374.json -n 42 -C 12`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-5b9da3d6.json -n 153 -C 24`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 267 -C 28`

## 2026-05-05 - CASS/artifact follow-up: older measured rejects

Scope: additional last-two-month CASS pass over the user-suggested negative
terms, then cross-checking older April artifact bundles that the CASS hits
pointed back toward. These are not broad design opinions; each item had a
measured reject or focused-test rollback.

- Mixed-OLTP record-header length microparser: replacing the serial-type length
  branch in `parse_record_header_into` with direct `SMALL_TYPE_SIZES` table use
  was rejected. The quick mixed baseline envelope was `1.399 s` and `1.425 s`,
  while candidate repeats were `1.390 s` and `1.518 s`; the average after-run
  was slower and the patch was rolled back. Evidence:
  `tests/artifacts/perf/20260424T2334Z-optimization-pass/RESULT.md`. Do not
  retry record-header length table reshuffling as an isolated mixed-OLTP lever;
  the later two-byte-header insert rejects reinforce that header microparsing
  only matters when a full matrix row moves.
- Delete sort insertion threshold: raising
  `sort_cells_desc_by_ptr::INSERTION_SORT_THRESHOLD` from `20` to `64` passed
  the focused sort-order proof but failed the wall-clock confirmation. The
  500-iteration delete run regressed from `5470.7 ms` to `5579.3 ms`, and the
  500-iteration `both` delete phase regressed from `1205.3 ms` to `1217.7 ms`.
  Evidence:
  `tests/artifacts/perf/20260427T2045Z-azurepine-delete-sort-threshold/RESULT.md`.
  Keep the threshold at `20`; do not tune it upward from a sort microbench
  without a delete/both e2e win.
- Delete large-N monotonic pre-scan removal: removing the pre-scan in
  `sort_cells_desc_by_ptr` improved local sort microbench cases, but the e2e
  `both` workload regressed within noise (`1.566 s` to `1.578 s`) and
  delete-only was only `1.01x +/- 0.03`, below the keep bar. Evidence:
  `tests/artifacts/perf/20260427T2235Z-snowyfortress-sort-prescan/RESULT.md`.
  Do not remove the pre-scan based on local sort timings; the accepted packed
  gap-shift path was the useful part of that pass.
- Early prepared direct INSERT zero-copy writer: an attempt to serialize
  prepared direct INSERT records directly into retained rightmost-leaf page
  space was fully rolled back before benchmarking because focused
  `direct_simple_insert` tests exposed unsafe retained/autocommit validation
  behavior (`29 passed`, `2 failed`). Evidence:
  `tests/artifacts/perf/20260428T0106Z-snowyfortress-post-compact/RESULT.md`.
  This is an earlier correctness-abandoned version of the later measured
  retained-leaf writer reject; do not re-enter this route without first
  isolating the retained/autocommit validation surface.

Primary CASS evidence that led back to these older bundles:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 120 -C 35`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-68d80f81.json -n 118 -C 24`

## 2026-05-05 - CASS follow-up: correctness-abandoned fast paths

Scope: last-60-day CASS search for the user-suggested negative terms. Direct
`--workspace /data/projects/frankensqlite` searches returned no hits for
`rejected`, `reverted`, `slower`, and `within noise`, so the follow-up searched
`frankensqlite <term>` and accepted only source paths or titles clearly tied to
this repo, especially `/home/ubuntu/.gemini/tmp/frankensqlite`.

- Prepared DML direct-VDBE execution bypass: a March optimization pass started
  changing prepared statements so DML could execute the stored `VdbeProgram`
  directly instead of re-entering `execute_statement_dispatch`, but abandoned
  the idea after reading the dispatch path. The reason is semantic, not just
  performance noise: DML dispatch owns trigger firing, FK enforcement,
  constraint handling, autocommit wrapping, and complex fallback routing. Do not
  retry by simply calling the precompiled VDBE program from
  `execute_prepared_with_params` for `INSERT`, `UPDATE`, or `DELETE`. A viable
  retry must first design a semantic-preserving prepared-DML executor that
  carries all trigger/FK/constraint/autocommit behavior, then prove it with
  DML correctness tests before any matrix benchmark.
- Whole-engine async/asupersync rewrite as an immediate perf lever: CASS
  contains conflicting March analyses, with one session arguing FrankenSQLite
  was leaving asupersync runtime benefits on the table and creating async VFS /
  pager / B-tree / VDBE migration beads, while a sibling session argued the
  synchronous `Cx` bridge is the intentional compatibility design. Treat this
  as architecture plan-space, not a rejected micro-optimization and not a
  substitute for current matrix profiling. Do not spend a performance campaign
  pass on "make the engine async" unless it is picked up as a tracked
  architecture epic with FFI/WASM compatibility, cancellation, and e2e logging
  gates.

Primary CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-ee1022e3.json -n 27 -C 6`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-52485ea5.json -n 13 -C 6`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-28-be5f24f8.json -n 9 -C 6`

## 2026-05-05 - Direct INSERT transient heap TEXT pooling

- Target: `INSERTThroughput` quick insert matrix, especially 10K single-txn
  medium/large record rows where `row_build_ns` spends milliseconds building
  concat-derived TEXT values.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-types/src/value.rs`.
- Candidate shape: expose the `SmallText` inline capacity, acquire a reusable
  heap `SqliteValue::Text` from the existing thread-local value pool for
  direct-simple INSERT concat chains, and return discarded transient row values
  to the same pool on write-only lazy MemDB paths.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-current-purplecoast-20260505T060835Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/direct-insert-text-pool-purplecoast-20260505T063845Z/report.json`.
  - Focused proof passed:
    `cargo test -p fsqlite-core test_prepared_direct_simple_insert_returns_transient_heap_text_to_pool --profile release-perf -- --nocapture`.
- Result: rejected and manually reverted before commit. The proof showed the
  write-only direct INSERT path could return a heap TEXT slot to the pool, but
  the real insert matrix moved the wrong way: average ratio worsened from
  `3.127x` to `3.226x`, geomean worsened from `2.894x` to `3.018x`, and the
  record-size `large_10col` 10K row regressed from `35.902 ms` to `42.537 ms`
  (`3.652x` to `4.068x` vs C SQLite). Do not retry this value-pool handoff
  unless a later design can prove lower per-row overhead and an insert-section
  A/B improves the primary ratios, not just a unit proof.

## 2026-05-05 - Pager sorted write-page append fast path

- Target: `INSERTThroughput` quick insert matrix, especially split-heavy 10K
  single-transaction rows where the pager maintains `write_pages_sorted` before
  WAL commit publication.
- Touched during rejected candidate:
  `crates/fsqlite-pager/src/pager.rs::insert_page_sorted`.
- Candidate shape: check the current last sorted page first, append when the
  new page number is greater, return on duplicate-last, and fall back to the
  existing binary-search insertion only for out-of-order page numbers.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/insert-sorted-page-append-cyangorge-20260505T1450Z/report.json`.
  - Candidate summary:
    `tests/artifacts/perf/insert-sorted-page-append-cyangorge-20260505T1450Z/summary.md`.
  - Focused pager sorted-order tests passed under `rch exec -- env
    CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-sorted-page-target cargo
    test -p fsqlite-pager sorted -- --nocapture`; `cargo fmt --check` also
    passed before the benchmark run.
- Result: rejected and manually reverted before commit. The primary weighted
  score worsened from `1.6991` to `1.7591`, average ratio from `2.4610x` to
  `2.5153x`, and geomean ratio from `2.3623x` to `2.4081x`. The important
  10K single-transaction rows did not produce a usable win: `small_3col`
  worsened from `6.895 ms` to `7.105 ms`, `large_10col` worsened from
  `36.165 ms` to `36.909 ms`, and only `medium_6col` improved
  (`13.666 ms` to `12.944 ms`). Do not retry this standalone
  append/equal-last `write_pages_sorted` micro-optimization unless a fresh
  profile shows sorted-page maintenance dominating and a full insert-section
  A/B improves the primary weighted score and the large-row medians.

## 2026-05-05 - WAL prepared-frame no-memset serializer

- Target: insert commit hot path where WAL frame preparation appeared to pay a
  payload-sized zero-fill before overwriting the full frame bytes.
- Touched during rejected candidate:
  `crates/fsqlite-wal/src/wal.rs::prepare_frame_bytes_with_transforms_into`.
- Candidate shape: replace `Vec::resize(total_bytes, 0)` plus frame overwrite
  with direct frame-byte appends via `push_wal_frame_bytes`, preserving checksum
  transform calculation while avoiding memset-style initialization.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/wal-no-memset-clean-baseline-cyangorge-20260505T063541Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/wal-no-memset-clean-candidate-cyangorge-20260505T063541Z/report.json`.
  - Focused proof passed:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-wal test_prepared_batch -- --nocapture`.
- Result: rejected and reverted by CyanGorge before commit. A clean-worktree A/B
  on `HEAD` (`5b5212f5`) improved insert average ratio from `3.184x` to
  `2.955x` and geomean from `2.915x` to `2.750x`, but the primary weighted
  score was effectively unchanged (`2.08110` to `2.07856`) and important rows
  regressed: write-single average ratio moved from `1.821x` to `1.868x`,
  `large_10col` 10K single-transaction F median moved from `36.58 ms` to
  `38.43 ms`, and 1000-row autocommit F median moved from `1.54 ms` to
  `1.83 ms`. Do not retry this serializer shape unless a fresh profile shows
  zero-fill dominates a current workload and a full section A/B improves the
  primary/weighted score without write-single regression.

## 2026-05-05 - Prepared indexed-equality schema microbatch carry

- Target: `Read-After-Write Query Performance`, especially repeated prepared
  secondary indexed equality probes.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: apply the existing prepared-statement microbatch
  schema-identity carry to `PreparedStatement::try_query_clean_memory_indexed_equality_fast`,
  mirroring the rowid query-row no-refresh path.
- Evidence:
  - Baseline/read context:
    `tests/artifacts/perf/read-point-pathtrace-cyangorge-20260505T0112Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/read-indexed-equality-microbatch-candidate-cyangorge-20260505T0131Z/report.json`.
  - Candidate repeat:
    `tests/artifacts/perf/read-indexed-equality-microbatch-candidate-repeat-cyangorge-20260505T0135Z/report.json`.
- Result: rejected before commit and reverted. A focused correctness proof
  showed the no-refresh indexed path could renew then carry the schema epoch,
  but the e2e read matrix did not produce a clean primary win. The first
  candidate run worsened the primary weighted score from `2.685x` to `2.995x`.
  The repeat improved the slowest 100K secondary-index ratio (`48.28x` to
  `33.06x`) and p90/p99, but still worsened the primary weighted score to
  `2.779x`; absolute FrankenSQLite secondary medians also regressed at 1K and
  10K rows.
- Do not retry the same schema-carry placement inside
  `try_query_clean_memory_indexed_equality_fast`. Reconsider only if a profile
  proves schema identity validation dominates repeated secondary probes and a
  close A/B read-section run improves the primary weighted score and
  FrankenSQLite absolute medians.

## 2026-05-05 - File-backed prepared indexed-equality last-result cache

- Target: prepared secondary indexed equality probes in the read benchmark.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: reuse `prepared_indexed_equality_last_result` in the
  file-backed `SimpleIndexedEqualityLookup` collection and `query_row` arms,
  with file-backed proof coverage for repeat-probe reuse and invalidation after
  external writes.
- Evidence:
  - Focused proof: `cargo test -p fsqlite-core test_file_backed_clean_prepared_indexed_equality_reuses_last_probe_until_external_write -- --nocapture`.
  - Baseline: `/data/tmp/frankensqlite-purplecoast-indexeq-base-read-20260505T0100Z.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-indexeq-candidate-read-20260505T005522Z.json`.
- Result: rejected and reverted before commit. The proof test passed, but the
  e2e read benchmark's secondary-index row uses `:memory:` and exits through
  `PreparedStatement::try_query_clean_memory_indexed_equality_fast`, so the
  candidate did not target the matrix path. Same-HEAD A/B artifacts were too
  noisy to defend as a real matrix win.
- Do not retry the file-backed last-result cache for the current read-section
  gap. Reconsider only for a workload that actually exercises file-backed
  prepared indexed equality, or after the benchmark target is proven to enter
  the file-backed branch.

## 2026-05-04 - Prepared COUNT(*) LIKE snapshot cache

- Target: `String & Pattern Matching Performance`, especially prepared
  `SELECT COUNT(*) FROM docs WHERE title/body LIKE <literal pattern>` rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  adjacent byte-compare cleanup in `crates/fsqlite-types/src/value.rs` landed
  separately.
- Candidate shape: add a one-entry `PreparedCountLikePatternLastResult` cache
  for clean-memory prepared `COUNT(*) WHERE col LIKE literal` query-row calls,
  keyed by root page, column, rowid alias, LIKE fast-path kind/literal, visible
  commit sequence, and MemDB undo version.
- Candidate commit:
  `b9cc83a7 perf(core): cache prepared COUNT(*) ... LIKE pattern results across clean-memory snapshots`.
- Revert commit: `a05d1e02 perf(core): revert regressed count-like cache`.
- Evidence:
  - Candidate/revert string artifacts:
    `tests/artifacts/perf/string-like-cache-candidate-cyangorge-20260504T2055Z/report.json`
    and
    `tests/artifacts/perf/string-like-cache-revert-cyangorge-20260504T2130Z/report.json`.
  - Earlier local candidate artifacts:
    `tests/artifacts/perf/string-like-count-cache-candidate-local-20260503T031439Z/report.json`
    and repeat
    `tests/artifacts/perf/string-like-count-cache-candidate-repeat-local-20260503T031459Z/report.json`.
- Result: rejected and reverted. The cache proof was plausible, but the real
  string-section benchmark did not produce a defensible matrix win and the
  landed cache was explicitly reverted as regressed. Do not retry the same
  one-entry prepared count-like result cache. Reconsider only if a fresh profile
  proves repeated `COUNT LIKE` result caching removes more work than
  schema/snapshot validation adds, and a close A/B string-section run improves
  FrankenSQLite absolute medians for prefix and wildcard rows without moving
  regressions into other string rows.

## 2026-05-05 - GROUP_CONCAT integer itoa append

- Target: string workload `GROUP_CONCAT` rows, especially
  `SELECT tag, GROUP_CONCAT(id, ',') FROM docs GROUP BY tag`.
- Touched during rejected candidate:
  `crates/fsqlite-func/src/agg_builtins.rs`,
  `crates/fsqlite-func/Cargo.toml`.
- Candidate shape: add `itoa` to `fsqlite-func` and format
  `SqliteValue::Integer` directly into the aggregate accumulator string instead
  of allocating through `to_text()` / `i64::to_string()`.
- Evidence:
  - Candidate: `/data/tmp/frankensqlite-purplecoast-groupconcat-candidate-string-20260505T0118Z.json`.
  - Same-window clean baseline: `/data/tmp/frankensqlite-purplecoast-groupconcat-base-string-20260505T0120Z.json`.
- Result: rejected before commit and manually reverted. Same-window
  FrankenSQLite medians worsened: 100 rows `77.1 us` to `242.8 us`, 1000 rows
  `701.7 us` to `725.1 us`, and 10000 rows `6.06 ms` to `8.85 ms`. The average
  string ratio stayed about `3.38x` and did not improve.
- Do not retry direct per-step `itoa::Buffer` formatting inside
  `GroupConcatFunc::step`. Reconsider only with a design that avoids per-row
  formatter setup and proves real string-section wins.

## 2026-05-05 - Positive-start ASCII-prefix SUBSTR fast path

- Target: `String & Pattern Matching Performance`, specifically
  `string functions (LENGTH + UPPER + SUBSTR)` rows.
- Touched: `crates/fsqlite-func/src/builtins.rs`.
- Candidate shape: for `SUBSTR(text, positive_start, positive_length)`, prove
  only the requested prefix is ASCII and slice by byte offset before the
  existing full-string `is_ascii()` / Unicode-count path.
- Candidate commit: `ee1649d5 perf(substr): ascii-prefix fast path for positive (start, length) substr`.
- Revert commit: `426590d5 perf(substr): revert rejected ascii-prefix fast path`.
- Evidence:
  - Baseline: `/data/tmp/frankensqlite-purplecoast-substr-prefix-base-string-20260505T0142Z.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-substr-prefix-candidate-string-20260505T0148Z.json`.
- Result: rejected and reverted. The candidate improved only the largest
  string-functions row slightly (`10000 rows` FrankenSQLite median `12.06 ms`
  to `11.84 ms`), while worsening smaller rows (`100 rows` `107.1 us` to
  `133.7 us`, `1000 rows` `1.23 ms` to `1.38 ms`) and worsening the string
  section average ratio from `3.17x` to `3.66x`.
- Do not retry as a per-call prefix probe in `SubstrFunc`. Reconsider only if a
  profile isolates `SUBSTR` body scanning as the dominant cost and a close A/B
  string-section run improves every affected string-functions row or the section
  score without small-row regression.

## 2026-05-05 - SmallText direct-byte Eq/Ord/Hash traits

- Target: `Read-After-Write Query Performance`, especially secondary indexed
  equality probes whose cache path compares/hashes short TEXT values.
- Touched: `crates/fsqlite-types/src/value.rs`.
- Candidate shape: make `SmallText` `PartialEq`, `Ord`, and `Hash` use
  `as_bytes_direct()` instead of `as_str()` so inline strings avoid repeated
  UTF-8 validation; preserve `str` hash compatibility by writing bytes plus the
  `0xff` separator used by `Hasher::write_str`.
- Evidence:
  - Baseline: `tests/artifacts/perf/read-indexed-baseline-cyangorge-20260504T2355Z/report.json`.
  - Noisy candidate: `tests/artifacts/perf/read-smalltext-byte-traits-cyangorge-20260505T0001Z/report.json`.
  - Candidate repeat after the competing build finished:
    `tests/artifacts/perf/read-smalltext-byte-traits-cyangorge-20260505T0010Z/report.json`.
- Result: rejected before commit and reverted. The candidate repeat did not move
  the read-section average (`3.09x` versus `3.08x` baseline). Secondary indexed
  lookup remained mixed: the 100-row fsqlite median was essentially unchanged,
  the 1000-row median worsened, and the 10000-row improvement was within noise
  while the row still had high variance.
- Do not retry as a broad `SmallText` trait cleanup. Reconsider only if a CPU or
  allocation profile shows UTF-8 validation inside `SmallText` traits dominating
  a specific workload and a clean A/B run improves FrankenSQLite absolute
  medians, not just C/FrankenSQLite ratios.

## 2026-05-05 - Direct REAL accumulator for rowid-bucket SUM GROUP BY

- Target: `Read-After-Write Query Performance`, especially
  `SUM + GROUP BY (~10 groups)` rows.
- Touched: `crates/fsqlite-vdbe/src/codegen.rs`.
- Candidate shape: for `SUM(<REAL NOT NULL column>)` grouped by a rowid bucket,
  replace generic `AggStep`/`AggFinal` dispatch with a direct `REAL 0.0`
  accumulator and `Add` opcode in the rowid-bucket sorter-bypass plan.
- Candidate commits: `7ec9d6b1 perf(codegen): direct REAL accumulator for GROUP BY rowid-bucket SUM`
  and `a0f674c6 test(codegen): swap rowid-bucket SUM test divisors back`.
- Evidence:
  - Baseline: `tests/artifacts/perf/read-indexed-baseline-cyangorge-20260504T2355Z/report.json`.
  - Candidate: `tests/artifacts/perf/read-groupby-direct-real-sum-cyangorge-20260505T0019Z/report.json`.
- Result: rejected and reverted. The 10000-row group row improved
  (`4.436 ms` to `3.888 ms`, ratio `3.44x` to `2.77x`), but the 1000-row
  group row regressed badly (`0.350 ms` to `0.800 ms`, ratio `2.77x` to
  `5.47x`), the 100-row group row slightly worsened, and the read-section
  average ratio worsened from `3.08x` to `3.56x`.
- Do not retry the direct accumulator as a narrow opcode substitution. Revisit
  only if a profile proves generic aggregate dispatch dominates all target group
  sizes and a close A/B read-section run improves the section score or every
  affected group row.

## 2026-05-05 - Direct single-rowid DELETE lowering

- Target: `UPDATE/DELETEThroughput`, especially prepared
  `DELETE FROM bench WHERE id = ?1`.
- Touched: `crates/fsqlite-vdbe/src/codegen.rs`.
- Candidate shape: when DELETE has a simple rowid equality predicate, skip the
  one-row `RowSetAdd`/`RowSetRead` two-pass plan and emit direct
  `SeekRowid`/`Delete` code, leaving non-rowid predicates on the two-pass path.
- Evidence:
  - Baseline: `tests/artifacts/perf/update-delete-current-cyangorge-20260505T0058Z/report.json`.
  - Candidate: `tests/artifacts/perf/update-delete-direct-delete-candidate-cyangorge-20260505T0100Z/report.json`.
- Result: rejected before commit and reverted. The average section ratio moved
  from `4.36x` to `4.03x`, but the targeted DELETE medians regressed at the
  smaller, high-signal sizes: `100 rows / delete 5 rows` worsened from
  `617.6 us` to `765.2 us`, and `1000 rows / delete 50 rows` worsened from
  `1.34 ms` to `1.58 ms`. The 10000-row DELETE improvement was only a small
  `10.30 ms` to `10.06 ms` move and does not justify the small-row loss.
- Do not retry as a simple RowSet skip. Reconsider only with an opcode-level
  profile proving RowSet overhead dominates DELETE and with a close A/B where
  FrankenSQLite DELETE medians improve at all three row counts.

## 2026-05-04 - Single-value insert serialization specialization

- Target: insert throughput, especially tiny/small single-column and small-record rows.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate commit: `7fa3f4d0 perf(record): specialize single-value insert serialization`.
- Revert commit: `5e9445ac Revert "perf(record): specialize single-value insert serialization"`.
- Evidence:
  - Baseline: `/data/tmp/frankensqlite-purplecoast-postcommit-parent-20260504T220353Z-report.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-postcommit-head-20260504T220353Z-report.json`.
- Result: rejected and reverted. Overall fsqlite geomean time changed by `1.0247x`
  slower, average time was `+3.89%`, with 11 improved rows and 14 regressed rows.
- Do not retry unless the exact insert section is benchmarked first and the
  implementation avoids adding overhead to multi-column insert rows.

## 2026-05-04 - Two-byte precomputed record header support

- Target: insert serialization for records whose serial types need two-byte varints.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: add `PrecomputedSerialTypeKind::AnyTwoByteVarint` and patch
  precomputed record headers at runtime.
- Evidence:
  - Candidate: `/data/tmp/frankensqlite-purplecoast-two-byte-record-candidate-20260504T2218Z-report.json`.
  - Baseline: `/data/tmp/frankensqlite-purplecoast-postcommit-parent-20260504T220353Z-report.json`.
- Result: rejected before commit. Overall fsqlite geomean time changed by
  `1.1139x` slower, average time was `+13.97%`, with 6 improved rows and
  19 regressed rows.
- Do not retry as a general record-header optimization. Only reconsider if a
  profile proves two-byte serial type patching is isolated to a workload where
  the end-to-end matrix improves.

## 2026-05-04 - Prepared PK rowid last-result cache

- Target: `Read-After-Write Query Performance`, especially `point lookup (PK)`.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: one-entry version-scoped cache for repeated prepared primary
  key rowid lookups, sharing invalidation keys with existing prepared MemDB
  caches.
- Evidence:
  - Full matrix that motivated the target: `/data/tmp/frankensqlite-purplecoast-current-full-20260504T2230Z-report.json`.
  - Candidate read section: `/data/tmp/frankensqlite-purplecoast-rowid-cache-candidate-read-20260504T2245Z-report.json`.
  - Close baseline read section: `/data/tmp/frankensqlite-purplecoast-rowid-cache-baseline-read-20260504T2252Z-report.json`.
  - Saved rejected patch: `/data/tmp/frankensqlite-purplecoast-rowid-cache-20260504T2252Z.patch`.
- Result: rejected before commit. The targeted correctness test passed, but the
  close A/B read geomean regressed from `2.41x` to `3.15x` versus C SQLite.
  PK fsqlite-time rows also regressed: `100 rows` by `1.15x`, `1000 rows` by
  `1.43x`, and `10000 rows` by `2.26x`.
- Do not retry the same one-entry rowid result cache. Reconsider only if the
  query-row dispatch path is redesigned so the cache removes more work than it
  adds, and prove it with a close A/B read-section run.

## 2026-05-04 - Unbounded grouped join rowid-count helper

- Target: join read rows, especially `JOIN + GROUP BY` and `JOIN + HAVING`.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: remove the small-right-table limit around the prepared inner
  join grouped rowid-count helper so larger right tables use the direct helper.
- Evidence:
  - Candidate: `tests/artifacts/perf/join-rowid-count-peer-candidate-cyangorge-20260504T1955Z/report.json`.
  - Baseline context from clean quick matrix at `a05d1e02`: `JOIN + GROUP BY`
    fsqlite median about `14.08 ms`; `JOIN + HAVING` about `13.97 ms`.
- Result: rejected before commit. Candidate focused join rows measured
  `17.42 ms` for `JOIN + GROUP BY` and `19.22 ms` for `JOIN + HAVING`, worse
  than the clean context despite the direct helper test shape.
- Do not retry by simply removing the row limit. Reconsider only if the helper
  is fed through the real prepared-query refresh path and a close A/B join run
  improves the actual matrix rows.

## 2026-05-04 - Standard-library ASCII LIKE byte comparison

- Target: string workload rows, especially LIKE prefix/contains/wildcard scans.
- Touched: `crates/fsqlite-types/src/value.rs`.
- Candidate shape: replace the local ASCII-case byte comparison helper with
  `[u8]::eq_ignore_ascii_case`.
- Evidence:
  - Baseline: `tests/artifacts/perf/string-clean-head-cyangorge-20260504T2240Z/report.json`.
  - Candidate: `tests/artifacts/perf/string-std-ascii-ci-cyangorge-20260504T2246Z/report.json`.
- Result: rejected before commit. Average string-section ratio worsened from
  about `3.03x` to `3.73x`; 100-row and 10K-row prefix/wildcard rows regressed,
  with only the 1K-row prefix case improving.
- Do not retry as a general LIKE matcher cleanup. Reconsider only with an
  end-to-end string-section A/B that shows row-level wins beyond noise.

## 2026-05-05 - Manual ASCII alpha bit-test in LIKE byte comparison

- Target: string workload rows, especially prepared `COUNT(*) ... LIKE`
  prefix/wildcard scans.
- Touched during rejected scratch candidate:
  `crates/fsqlite-types/src/value.rs`.
- Candidate shape: replace `u8::is_ascii_alphabetic()` in
  `fsqlite_types::ascii_ci_eq_byte` with a branchless-style
  `(byte | 0x20).wrapping_sub(b'a') <= b'z' - b'a'` helper. This was narrower
  than the previously rejected standard-library `eq_ignore_ascii_case`
  substitution.
- Evidence:
  - Correctness: `cargo test -p fsqlite-types like --release` passed in the
    clean detached worktree.
  - Baseline:
    `/data/tmp/frankensqlite-purplecoast-clean-20260505T032950Z/tests/artifacts/perf/string-clean-purplecoast-20260505T0330Z/report.json`.
  - Candidate:
    `/data/tmp/frankensqlite-purplecoast-clean-20260505T032950Z/tests/artifacts/perf/string-ascii-alpha-bit-candidate-purplecoast-20260505T0340Z/report.json`.
- Result: rejected before commit and reverted in scratch. The focused string
  matrix worsened from `3.37x` average ratio to `3.63x`; key FrankenSQLite
  medians regressed: 10K prefix LIKE `2.32 ms` to `2.78 ms`, 10K wildcard LIKE
  `3.42 ms` to `3.70 ms`, and 10K GROUP_CONCAT `6.64 ms` to `8.29 ms`.
- Do not retry bit-test microcleanup unless a future compiler/codegen profile
  proves this exact helper dominates LIKE matching.

## 2026-05-04 - Exact-sized record body writes

- Target: record-size insert section, especially `large_10col`.
- Touched: `crates/fsqlite-types/src/record.rs`.
- Candidate shape: pre-size the serialized record buffer to the full record size
  and write payload bytes into exact slices instead of appending payload bytes.
- Evidence:
  - Baseline: `tests/artifacts/perf/record-current-clean-cyangorge-20260504T2300Z/report.json`.
  - Candidate: `tests/artifacts/perf/record-exact-body-write-cyangorge-20260504T2300Z/report.json`.
- Result: rejected before commit. Tiny rows improved, but small/medium/large
  FrankenSQLite medians regressed; the section only appeared better because the
  C SQLite large-row sample slowed down.
- Do not retry the same exact-body `Vec::resize` strategy unless a profile proves
  payload append/copy dominates and a close A/B record-section run improves the
  actual FrankenSQLite medians.

## 2026-05-04 - Two-byte runtime precomputed record headers, repeat

- Target: record-size insert section, especially medium/large rows with long
  TEXT serial types.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: add a two-byte runtime precomputed-header slot for direct
  inserts whose first row has long TEXT/BLOB serial types.
- Evidence:
  - Baseline: `tests/artifacts/perf/record-current-clean-cyangorge-20260504T2300Z/report.json`.
  - Candidate: `tests/artifacts/perf/record-two-byte-runtime-header-cyangorge-20260504T2315Z/report.json`.
  - Candidate repeat: `tests/artifacts/perf/record-two-byte-runtime-header-repeat-cyangorge-20260504T2320Z/report.json`.
- Result: rejected before commit. The repeat showed tiny/medium improvements but
  large-row FrankenSQLite time regressed from the clean baseline, and the ratio
  improvement was mostly from a slower C SQLite large-row sample.
- Do not retry as a broad runtime-header extension. Only revisit if two-byte
  patching is isolated to a proven row shape and judged on FrankenSQLite absolute
  time as well as C/FrankenSQLite ratio.

## 2026-05-05 - MemoryVfs contiguous batch append

- Target: insert throughput rows, especially explicit single-transaction
  `large_10col` and record-size insert rows where profiling showed commit
  roundtrip dominated by many dirty memory pages.
- Touched during rejected candidate: `crates/fsqlite-vfs/src/memory.rs`.
- Candidate shape: keep existing `MemoryFile::write_page_batch` reservation and
  accounting, but process normalized writes in order so contiguous append
  suffixes use `Vec::extend_from_slice` instead of resizing the whole final
  file length to zero and then copying dirty pages over it.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-cyangorge-20260505T044600Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/insert-memoryvfs-batch-append-candidate-cyangorge-20260505T050100Z/report.json`.
  - Correctness: `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-vfs write_page_batch -- --nocapture`
    passed the three focused `write_page_batch` tests.
- Result: rejected before commit and reverted. Insert-only average ratio
  worsened from `2.77x` to `3.12x`; `large_10col` 10K single-transaction
  FrankenSQLite median regressed from `37.81 ms` to `44.58 ms`, and the
  profile hook showed `commit_roundtrip_ns` for record-size `large_10col`
  remained essentially unchanged/slightly worse (`15.98 ms` to `16.42 ms`).
- Do not retry this as a MemoryVfs microcopy cleanup. Reconsider only if a
  lower-level profile proves `Vec::resize` zero-fill is still a top self-time
  frame and a close insert-section A/B improves FrankenSQLite absolute medians,
  not just ratio noise.

## 2026-05-05 - Prepared direct insert retained-leaf writer append

- Target: insert throughput rows, especially explicit single-transaction
  `large_10col` and record-size comparison rows where the profile showed
  serialization plus B-tree cell assembly still visible under the direct insert
  path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`,
  `crates/fsqlite-btree/src/cursor.rs`.
- Candidate shape: route prepared monotonic direct inserts through writer
  callbacks (`table_append_after_last_position_with_writer` plus a retained
  `TableAppendHint` writer analogue) and exact-size record slice serializers so
  the record bytes are written directly into the reserved leaf cell instead of
  first materializing `record_scratch`.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-cyangorge-20260505T044600Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/insert-writer-candidate-cyangorge-20260505T0545Z/report.json`.
  - Correctness: `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo check -p fsqlite-core -p fsqlite-btree`
    passed before measurement.
  - Correctness: `cargo test -p fsqlite-btree test_cached_rightmost_leaf_hint_with_writer_updates_retained_hint -- --nocapture`
    passed; the RCH wrapper later had to be killed while retrieving artifacts.
  - Correctness: `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`
    passed.
- Result: rejected after commit and reverted by follow-up commit. Insert-only
  average ratio worsened from `2.77x` to `3.10x`. The 10K single-transaction
  `large_10col` FrankenSQLite median regressed from `37.81 ms` to `42.26 ms`;
  the record-size `large_10col` FrankenSQLite median regressed from `40.37 ms`
  to `42.89 ms`. The profile showed the root cause: record serialization did
  shrink on the record-size `large_10col` path (`serialize_ns` about `1.74 ms`
  to `1.40 ms`), but B-tree insert time grew much more (`btree_insert_ns` about
  `7.91 ms` to `12.52 ms`) because the writer route added extra append
  preflight/callback overhead on the hot leaf path.
- Do not retry the retained-leaf writer callback as a general direct insert
  optimization. Reconsider only if the B-tree writer path can preflight room
  without duplicate layout work on full leaves and a close insert-section A/B
  improves FrankenSQLite absolute medians, not just serialization counters.

## 2026-05-05 - Explicit :memory: concurrent transaction retained writer

- Target: explicit single-transaction INSERT and UPDATE/DELETE rows where
  benchmark-shaped private `:memory:` workloads pay fixed BEGIN/COMMIT ceremony
  between logical transactions.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: reuse the existing committed cached writer machinery across
  explicit private-memory concurrent transactions. `COMMIT` would call
  `commit_and_retain()` and park the committed writer; the next default
  explicit `BEGIN` would take that cached writer while still registering a fresh
  MVCC concurrent session.
- Evidence:
  - Correctness proof attempted:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-local-target cargo test -p fsqlite-core test_memory_explicit_concurrent_commit_parks_and_reuses_writer -- --nocapture`
  - The focused proof failed on the second `COMMIT` with
    `BusySnapshot { conflicting_pages: "2" }` after the second transaction
    wrote the same table root page. The first retained commit appeared to park,
    and the second `BEGIN` appeared to register a distinct concurrent session,
    but FCW still treated page 2 as too new for the second logical transaction.
- Result: rejected before any benchmark. The code was reverted because it
  violated the explicit concurrent transaction visibility model. The failure is
  a correctness blocker, not a tuning tradeoff.
- Do not retry by simply allowing explicit `BEGIN` to reuse `cached_write_txn`.
  A viable version would first need a proof that the retained pager handle's
  published snapshot, the new `ConcurrentRegistry` session snapshot, and the
  `concurrent_commit_index` frontier are all advanced together before any page
  write is tracked.

## 2026-05-05 - Precomputed record-header append serializer

- Target: quick INSERT matrix, especially cached-header direct INSERT rows where
  record serialization and allocation/copy cost still show up in the profile.
- Touched during rejected candidate: `crates/fsqlite-types/src/record.rs`.
- Candidate shape: for stack-sized `PrecomputedRecordHeader` serializers, stop
  pre-sizing the whole output record with zeroes. Instead, append the cached
  header template and then append serialized payload bytes with
  `append_serialized_value`. The first draft accidentally used
  `Vec::reserve(total_size - capacity)` after `clear()`, which can under-reserve
  because `reserve` is relative to length; the final measured candidate fixed
  that to reserve against the cleared vector length before benchmarking.
- Evidence:
  - Correctness:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-types precomputed_header -- --nocapture`
    passed.
  - Candidate build:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-measure-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
    passed in the detached measurement worktree.
  - Same-window clean baseline:
    `tests/artifacts/perf/record-precomputed-append-samewindow-baseline-cyangorge-20260505T0732Z/report.json`.
  - Final corrected candidate:
    `tests/artifacts/perf/record-precomputed-append-reserve-fixed-quick-candidate-cyangorge-20260505T0723Z/report.json`.
- Result: rejected and reverted. The final candidate lost to the same-window
  clean baseline on the insert quick matrix: primary weighted score worsened
  from `1.9105` to `1.9905`, average ratio worsened from `2.9409x` to
  `3.0146x`, and the row-level comparison had 13 FrankenSQLite medians
  regressing by more than 3% versus only one improving. The largest observed
  FrankenSQLite median regressions were medium_6col 100 rows (`0.432 ms` to
  `0.578 ms`), medium_6col 1000 rows (`1.606 ms` to `1.836 ms`), and
  medium_6col record-size 10K (`9.671 ms` to `10.628 ms`).
- Do not retry this zero-fill avoidance shape for cached precomputed record
  headers. Reconsider only if a lower-level profile proves `Vec::resize`
  zero-fill is a dominant self-time frame and a same-window A/B improves
  FrankenSQLite absolute medians, not just ratio noise against C SQLite.

## 2026-05-05 - VDBE concurrent-context borrow in stale page-one clear

- Target: update/delete write rows where `clear_stale_synthetic_pending_commit_surface`
  appeared as visible self-time under `SharedTxnPageIo::write_page_internal`.
- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: inside `clear_stale_synthetic_pending_commit_surface`, borrow
  `self.concurrent` once and use `as_ref()` instead of calling
  `self.concurrent_context()`, avoiding a `ConcurrentContext` clone on every
  stale synthetic page-one cleanup.
- Evidence:
  - Baseline update/delete profiles:
    `tests/artifacts/perf/update-delete-update-profile-cyangorge-20260505T0824Z/`
    and
    `tests/artifacts/perf/update-delete-delete-profile-cyangorge-20260505T0819Z/`.
  - Candidate profile:
    `tests/artifacts/perf/update-clear-context-borrow-candidate-cyangorge-20260505T0835Z/`.
  - Focused A/B:
    `tests/artifacts/perf/update-clear-context-borrow-ab-cyangorge-20260505T0843Z/hyperfine-update.json`.
  - Quick update baseline/candidate:
    `tests/artifacts/perf/update-clear-context-borrow-comprehensive-baseline-cyangorge-20260505T0848Z/report.json`
    and
    `tests/artifacts/perf/update-clear-context-borrow-comprehensive-candidate-cyangorge-20260505T0853Z/report.json`.
  - Quick insert candidate:
    `tests/artifacts/perf/update-clear-context-borrow-insert-candidate-cyangorge-20260505T0858Z/report.json`,
    compared against same-code clean insert baseline
    `tests/artifacts/perf/record-precomputed-append-samewindow-baseline-cyangorge-20260505T0732Z/report.json`.
  - Correctness:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-clean-cyangorge-target-20260505T0815Z RUSTFLAGS='-C force-frame-pointers=yes' cargo test -p fsqlite-vdbe shared_txn_page_io --profile release-perf -- --nocapture`
    passed in the detached measurement worktree.
- Result: rejected and reverted. The focused update/delete probe looked
  promising: `perf-update-delete 10000 40 update` improved from `1969 ns` to
  `1851 ns` per updated row, the focused hyperfine mean improved about `2.1%`,
  and the quick update section geomean ratio improved from `3.8912x` to
  `3.3689x`. The broader insert quick section failed the keep bar: the
  candidate's insert average ratio worsened from `2.9409x` to `2.9584x`, the
  geomean worsened from `2.6920x` to `2.7167x`, and FrankenSQLite absolute
  medians regressed across nearly every insert row, including medium_6col
  100 rows (`0.432 ms` to `0.572 ms`), small_3col 1000 rows (`1.013 ms` to
  `1.151 ms`), and record-size large_10col 10K (`34.98 ms` to `37.87 ms`).
- Do not retry this clone-avoidance borrow change as a standalone hot-path
  cleanup. Reconsider only if a same-window insert and update A/B both improve
  FrankenSQLite absolute medians, or if the stale page-one cleanup is isolated
  away from insert-heavy write paths.

## 2026-05-05 - B-tree staged-page mutation for same-size UPDATE overwrite

- Target: direct simple UPDATE rows where
  `BtCursor::table_overwrite_current_payload_same_size_no_overflow` appeared
  under the update profile and wrote an already-staged leaf page back through
  `write_page_data`.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`.
- Candidate shape: after validating the current leaf-table cell and patching
  the cursor stack page image, call `PageWriter::try_mutate_staged_page_data`
  to patch the transaction-owned staged page payload in place. This avoided the
  full-page `write_page_data` path when the same page had already been staged
  by an earlier update in the transaction.
- Evidence:
  - Correctness:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-btree-target cargo test -p fsqlite-btree table_overwrite_current_payload_same_size_no_overflow -- --nocapture`
    passed both focused overwrite tests, including the added staged-page proof.
    RCH then hung retrieving target artifacts and was interrupted after the
    successful test result was printed.
  - Build:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-btree-local-target RUSTFLAGS='-C force-frame-pointers=yes' cargo build -p fsqlite-e2e --bin perf-update-delete --profile release-perf`
    passed.
  - A/B artifact:
    `tests/artifacts/perf/btree-same-size-overwrite-cyangorge-20260505T0755Z/hyperfine-update.json`.
  - Corrected same-code A/B artifact after a concurrent peer commit landed:
    `tests/artifacts/perf/btree-same-size-overwrite-current-head-cyangorge-20260505T0804Z/hyperfine-update.json`.
- Result: rejected and reverted. The preliminary A/B showed the baseline ahead,
  but it used a clean binary from before a concurrent peer commit. The corrected
  current-code A/B on the exact update workload,
  `perf-update-delete 10000 40 update`, was still a no-win: clean baseline mean
  `357.9 ms +/- 6.1 ms`, candidate mean `359.4 ms +/- 7.4 ms`, with hyperfine
  reporting the baseline as `1.00 +/- 0.03` times faster. The extra staged-page
  mutation hook and second payload copy did not clear the keep bar against the
  existing full-page overwrite-steal path.
- Do not retry staged-page mutation for same-size UPDATE as a standalone B-tree
  change. Reconsider only if the direct UPDATE caller can supply a payload-slice
  patch that avoids rebuilding the full record first, or if a profile shows
  `write_page_data` copying itself dominates after connection-level payload
  construction is removed.

## 2026-05-05 - VDBE IntDivide opcode for rowid-bucket GROUP BY

- Target: remaining read-aggregate gap, especially
  `100 rows / SUM + GROUP BY (~10 groups)`.
- Touched during rejected candidate: `crates/fsqlite-types/src/opcode.rs`,
  `crates/fsqlite-vdbe/src/lib.rs`, `crates/fsqlite-vdbe/src/engine.rs`,
  and `crates/fsqlite-vdbe/src/codegen.rs`.
- Candidate shape: add a custom `Opcode::IntDivide`, emitted only by
  `codegen_select_group_by_rowid_bucket_sum`, to fast-path already-integer
  `rowid / divisor` before falling back to ordinary `Divide` semantics.
- Evidence:
  - Correctness:
    `cargo fmt --check` passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-types opcode_ -- --nocapture`
    passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-vdbe rowid_bucket -- --nocapture`
    passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-vdbe divide -- --nocapture`
    passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-vdbe division -- --nocapture`
    passed.
  - Same-host A/B reports:
    `tests/artifacts/perf/read-groupby-intdivide-clean-current-peer-baseline-purplecoast-20260505T082235Z/report.json`
    and
    `tests/artifacts/perf/read-groupby-intdivide-candidate-current-peer-purplecoast-20260505T082725Z/report.json`.
  - Repeat remote run log:
    `tests/artifacts/perf/read-groupby-intdivide-repeat-purplecoast-20260505T0926Z/run.log`.
    RCH did not retrieve the ignored `tests/artifacts/.../report.json`, so
    treat this as corroborating log evidence only, not the primary artifact.
- Result: rejected and reverted. The same-host read weighted score improved
  from `0.25776` to `0.24784`, but the targeted FrankenSQLite medians did not
  justify a new opcode: 100-row group-by improved only `0.022081 ms` to
  `0.021861 ms`, 1000-row group-by improved only `0.119825 ms` to
  `0.119293 ms`, and 10000-row group-by regressed from `1.111733 ms` to
  `1.162087 ms`. The apparent section-score and ratio wins were mostly C
  SQLite timing noise and unrelated read-single movement, while the remaining
  100-row group-by gap stayed open.
- Do not retry this by adding a narrow arithmetic opcode or by special-casing
  `Divide` dispatch for the rowid-bucket aggregate path. Reconsider only if a
  fresh bytecode profile proves division dispatch itself dominates the current
  workload and a same-window A/B improves FrankenSQLite absolute medians at
  all row counts plus the read-section weighted score.

## 2026-05-05 - Explicit transaction retained count/sum insert hook early return

- Target: insert throughput e2e matrix, especially explicit
  single-transaction insert rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: return early from
  `retained_autocommit_count_sum_cache_note_insert` when
  `self.in_transaction.get()` is true, on the theory that retained autocommit
  count/sum cache maintenance is irrelevant inside explicit transactions.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-countsum-explicit-baseline-cyangorge-20260505T0925Z/report.json`.
  - First candidate:
    `tests/artifacts/perf/insert-countsum-explicit-candidate-cyangorge-20260505T0931Z/report.json`.
  - Repeat baseline:
    `tests/artifacts/perf/insert-countsum-explicit-repeat-cyangorge-20260505T0932Z-baseline/report.json`.
  - Repeat candidate:
    `tests/artifacts/perf/insert-countsum-explicit-repeat-cyangorge-20260505T0933Z-candidate/report.json`.
- Result: rejected and reverted. The first pass looked mildly positive, but
  the repeat run failed the keep bar. Repeat candidate worsened primary
  weighted score from `1.9154` to `1.9516`, geomean ratio from `2.6390x` to
  `2.7181x`, FrankenSQLite absolute geomean from `2.3051 ms` to
  `2.3575 ms` (`+2.28%`), and FrankenSQLite absolute average from
  `6.3954 ms` to `6.5695 ms` (`+2.72%`). The largest repeat regression was
  record-size comparison 10K large_10col, `35.059 ms` to `37.517 ms`
  (`+7.01%`).
- Do not retry this as a standalone branch-elision micro-optimization.
  Reconsider only if retained autocommit cache maintenance is redesigned or a
  profile shows this exact hook dominating a retained-autocommit-only workload.

## 2026-05-05 - Exact transaction-control `execute` parse bypass

- Target: insert throughput e2e matrix, especially explicit
  single-transaction insert rows that call `execute("BEGIN;")` and
  `execute("COMMIT;")`.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: add an exact-string fast path in `Connection::execute` for
  `BEGIN`, `BEGIN;`, `COMMIT`, `COMMIT;`, `ROLLBACK`, and `ROLLBACK;`, calling
  the existing direct transaction helpers after `background_status()` and
  incrementing `note_connection_statement_execution_count(1)` only after the
  operation succeeds.
- Evidence:
  - Correctness proof passed before rejection:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-exact-txn-test-target cargo test -p fsqlite-core test_execute_exact_transaction_controls_skip_sql_parse_and_count_success -- --nocapture`
    showed zero parser calls and correct successful-execution stats. RCH then
    hung in post-test target artifact retrieval; the test body itself passed.
  - Existing guard still passed:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-exact-txn-test-target cargo test -p fsqlite-core test_file_backed_begin_transaction_api_skips_sql_parse -- --nocapture`.
  - Same-window baseline log:
    `tests/artifacts/perf/insert-exact-txn-baseline-purplecoast-20260505T101018Z/run.log`.
  - Same-window candidate log:
    `tests/artifacts/perf/insert-exact-txn-candidate-purplecoast-20260505T103455Z/run.log`.
    RCH did not retrieve the ignored JSON reports for this run, so treat these
    logs as the measurement artifact.
- Result: rejected and reverted. The local proof was real, but the matrix did
  not move in the right direction. Average time ratio worsened from `2.36x` to
  `2.55x`. Targeted FrankenSQLite medians were mixed or worse: single-txn
  tiny_1col 100 rows regressed from `299.9 us` to `336.1 us`, 1000 rows
  improved only from `836.0 us` to `805.4 us`, and 10000 rows regressed from
  `4.65 ms` to `4.87 ms`. Transaction-strategy small_3col single-txn rows
  regressed at all measured sizes: `219.1 us` to `267.1 us`, `1.04 ms` to
  `1.08 ms`, and `6.81 ms` to `7.12 ms`.
- Do not retry exact transaction-control parse bypass as a standalone
  optimization. Reconsider only if fresh profiles show `BEGIN`/`COMMIT` SQL
  parsing itself dominates the current insert workload and a repeated
  same-window A/B improves the absolute FrankenSQLite medians plus the
  insert-section score.

## 2026-05-05 - CASS last-60-day no-retry expansion

Scope: follow-up `cass` archaeology over the last 60 days, using a session set
from direct `/data/projects/frankensqlite` hits plus archived
`/home/ubuntu/.gemini/tmp/frankensqlite` sessions, then searching negative
signals including `rejected`, `reverted`, `abandoned`, `slower`,
`didn't help`, `did not help`, `no improvement`, `within noise`,
`regressed`, `worse`, `rollback`, `failed to improve`, `no measurable`, and
`revert it for now`. The attempted `cass index --json` refresh timed out in
the preparing phase, so these are evidence from the existing CASS index.

- Do not revive the `SqliteValue` `Arc<str>` / `Arc<[u8]>` conversion as a
  prerequisite for `Opcode::SCopy`, sorter, pseudo-cursor, or row-cache work.
  CASS shows it was attempted during the sorter/column-cache optimization pass,
  caused widespread cross-crate breakage, and was explicitly reverted back to
  `String`/`Vec<u8>` to regain a compilable state. This reinforces the older
  generic `SqliteValue` `Arc` entry: retry only with a designed serde and
  cross-crate migration plan, not as a local VDBE hot-path patch.
- Do not implement prepared DML execution by simply calling the compiled VDBE
  program and bypassing `execute_statement_dispatch`. CASS records the agent
  rejecting that shape after tracing DML dispatch: triggers, foreign keys,
  constraint enforcement, autocommit wrapping, and fallback paths live there.
  The acceptable shape is a precompiled-program hook that still preserves DML
  dispatch semantics; a direct bytecode-only shortcut is a correctness trap.
- Do not change the public `Row` representation from `Vec<SqliteValue>` to
  `SmallVec` as a standalone allocation optimization. CASS shows that the
  public-row `SmallVec` idea was reverted for API stability while keeping the
  internal VDBE `SmallVec` paths. Reconsider only with an explicit public API
  migration plan and downstream compatibility proof.
- Do not use the old raw-string `bench_insert` benchmark as the keep/reject
  proof for engine-level insert changes. CASS records an optimization pass that
  attacked serializer, VFS append, and hash-map hotspots but moved the benchmark
  only from about `0.271 s` to `0.265 s` because the benchmark itself generated
  10,000 distinct SQL strings and thrashed parse/codegen caches. Use the current
  prepared-statement matrix rows, or a same-window prepared insert microbench,
  before keeping engine patches.
- Treat `Opcode::MustBeInt`, `BtCursor::last` `at_eof`, active-transaction
  checkpoint blocking, and `with_pager_write_txn` active-transaction bypass as
  CASS false leads, not optimization targets. The mined sessions re-read those
  paths and concluded the current implementations were already handling the
  suspected issue or that the target was not a performance defect.

CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 204 -C 45`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 230 -C 80`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 120 -C 45`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-ee1022e3.json -n 30 -C 25`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-854547a1.json -n 140 -C 45`

## 2026-05-05 - CASS/git follow-up: reverted fast paths not yet named

Scope: another `cass` pass over the last 60 days restricted to FrankenSQLite
signals (`cass search "frankensqlite <term>" --days 60 ...`) for `reverted`,
`rollback`, `slower`, `worse`, `abandon*`, and related wording. The useful
new leads were then cross-checked against recent revert commits and preserved
artifact bundles. These entries are intentionally terse because they mainly
serve as search handles for future agents.

- `ensure_storage_cursor_row_layout` early-return fast path: reverted by
  `9dd7bc53`. The premise that a non-empty row decode table plus a large enough
  payload buffer meant the layout was reusable was false: multi-row cursor
  callers relied on the slow path to reset eager-value state. Do not re-add an
  early return here unless the guard also proves prior-row eager values cannot
  leak, with correctness coverage before any read benchmark.
- Prepared indexed-equality text/null side maps: reverted by `53679a91`
  (`7d9814e5`). The idea added `SmallText` and NULL-specific rowid maps beside
  the generic `PreparedIndexedEqualityCache`, but was dropped before becoming a
  durable read win. This is distinct from the later last-result cache rejects:
  do not retry by adding parallel value-shape maps unless a profile proves
  generic lookup-key construction dominates and a read-section A/B improves
  absolute FrankenSQLite medians.
- B-tree cell-slot cache rotation experiment: reverted by `facba056`.
  Replacing remove/insert LRU promotion with slice rotation and special
  in-entry slot updates did not survive the measured/reviewed perf pass. Keep
  the simpler current promotion path; do not retry cache-order micro-rotation
  without a profile showing `CellSlotCache` promotion itself is hot and a join
  or read-index A/B win.
- VDBE index-prefix binary compare shortcut: rejected by `f7fce439`. The
  candidate bypassed the collation registry for apparently binary index
  prefixes, then was removed in favor of the single registry-backed
  `compare_index_prefix_keys` path. Do not retry a registry-free prefix compare
  unless the collation and DESC/null semantics are proven with focused tests and
  the index-boundary/read-query artifacts show a real row-level win.
- Prepared rowid-bucket `SUM` fast path family: reverted by `6d8a44f4` after
  the initial `SimpleGroupByRowidBucketSum` helper and later streaming variant
  failed the keep bar. Artifacts include
  `tests/artifacts/perf/read-after-write-group-by-rowid-bucket-sum-candidate-calm-20260503T2008Z/report.json`
  and
  `tests/artifacts/perf/read-after-write-rowid-bucket-stream-candidate-calm-20260503T2018Z/report.json`.
  Do not recreate a whole prepared fast-path variant for `rowid / divisor`
  grouped `SUM` unless all row counts and the read-section score improve, not
  just the largest row.
- UPDATE reinsert existence-probe skip: reverted by `8dd631d7`. The candidate
  skipped the existence probe when reinserting the same rowid during UPDATE,
  but the update/delete section was worse than the disabled comparison
  (`4.1226` weighted score versus `3.7545` in
  `tests/artifacts/perf/update-delete-reinsert-skip-candidate-chartreuse-20260504T0057Z/report.json`
  and
  `tests/artifacts/perf/update-delete-reinsert-skip-disabled-dirty-chartreuse-20260504T0101Z/report.json`).
  Do not retry this as a local `PendingUpdateRestore` shortcut without an
  UPDATE-only A/B win and conflict/unique-index coverage.
- Top-category CTE rowid-carry regression: reverted by `86944a1b`. The
  candidate carried rowids for top categories through the direct CTE helper and
  then had to be unwound back to the simpler rescan-by-category shape. Evidence
  lives in
  `tests/artifacts/perf/subquery-current-head-cte-rowid-carry-local-20260501T0523Z/`
  and
  `tests/artifacts/perf/subquery-current-head-cte-rowid-carry-reverted-local-20260501T0530Z/`.
  Do not retry by preserving per-category rowid vectors unless the subquery/CTE
  row improves in a same-window run and memory growth is bounded.
- Prepared ORDER BY LIMIT winner-maintenance path: rejected by `3bfd8fa1` and
  removed again by `0cb0379e`. The candidate kept the winners vector sorted via
  `partition_point` / insert on every replacement; the reverted shape returned
  to unsorted winner replacement plus one final sort. Do not retry per-row
  sorted winner insertion for the prepared ORDER BY LIMIT path unless a
  same-window read/order benchmark proves the maintenance cost is hot and
  absolute FrankenSQLite medians improve.
- Stack-layout record serializer cache: reverted by `be75bb57`. The candidate
  added fixed stack arrays for up to 16 values in `serialize_record_iter_into_impl`
  to cache value refs, serial types, and payload lengths, then was removed from
  `crates/fsqlite-types/src/record.rs`. Do not retry this stack-layout serializer
  cache as a generic record-write optimization; use the existing record
  serializer entries and require a record/insert matrix win before reintroducing
  stack layout state.
- Integer-key fast path for inner-join grouped aggregate: reverted by
  `19f0b188`. The candidate added `memdb_integer_join_key_with_source`,
  `PreparedJoinGroupState`, and an integer-key grouped-join implementation
  beside the generic hash-key path, then was dropped back to the generic grouping
  flow. Do not retry a separate integer-only join grouping path unless join
  artifacts show generic `HashableJoinKey` construction dominates and all
  affected grouped-join rows improve.
- Direct DML cursor scratch routing: reverted by `80777b6b`; artifact bundle
  `tests/artifacts/perf/20260428T1743Z-sapphirecrane-direct-dml-cursor-scratch/RESULT-direct-dml-cursor-scratch.md`
  was preserved. This reinforces the existing direct-DML scratch no-retry rule:
  do not route INSERT/UPDATE/DELETE through shared cursor scratch as a local
  hot-path cleanup without a full correctness and update/delete matrix proof.

## 2026-05-05 - Conservative WAL raw append for large INSERT commits

Scope: `comprehensive-bench --quick --filter insert`, targeting the default
conservative WAL path in
`crates/fsqlite-pager/src/pager.rs::commit_wal_group_commit_with_snapshot`
after insert profiling showed 2014-frame `large_10col` commits spending
several milliseconds in prepared-frame construction and WAL append.

- Candidate shape: when `ParallelWalFallbackReason::OperatorForced` selected
  conservative mode and no lane-prepared batch was available, skip
  `wal.prepare_append_frames` / `finalize_prepared_frames` and fall through to
  the existing fused `wal.append_frames` raw append path.
- Evidence: baseline
  `tests/artifacts/perf/insert-profile-after-wal-default-cyangorge-20260505T1022Z/report-insert-profile.json`;
  candidate
  `tests/artifacts/perf/insert-profile-after-wal-default-cyangorge-20260505T1022Z/report-insert-raw-conservative-candidate.json`;
  candidate profile
  `tests/artifacts/perf/insert-profile-after-wal-default-cyangorge-20260505T1022Z/run-insert-raw-conservative-candidate.log`.
- Result: rejected and reverted. Insert geomean worsened `2.384x -> 2.444x`;
  write-bulk geomean worsened `2.546x -> 2.623x`; p99 worsened
  `4.301x -> 4.460x`. The motivating large rows regressed badly:
  `Single Transaction large_10col 10000` F median `35.404 ms -> 43.130 ms`,
  and `Record Size Comparison large_10col 10K` F median
  `34.613 ms -> 49.192 ms`.
- Do not retry raw conservative WAL append as a standalone prepared-batch
  bypass. Revisit only if a new design preserves prelock prepared-frame
  construction while reducing its transform/buffer cost, and proves the
  large-row insert section improves in a same-window matrix.

## 2026-05-05 - Private `:memory:` WAL commit bypass

Scope: `comprehensive-bench --quick --filter insert`, targeting private
`/:memory:` pager commits in `crates/fsqlite-pager/src/pager.rs` after insert
profiles showed large-row single-transaction commits dominated by dirty-page
publication.

- Candidate shape: in a clean temporary worktree based on `71b6720f`, route
  `memory_db_bump_alloc` commits through direct private-memory page flushing
  before the WAL branch, skip WAL conflict prediction for private memory, and
  avoid synthetic page-1 rewrites for ordinary private-memory growth unless
  page 1 or the freelist was actually dirty. The candidate also made
  `commit_and_retain` defer private-memory VFS flushing when the retained
  writer could publish committed pages through its retained cache.
- Evidence:
  - Focused proof:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-memcommit-target cargo test -p fsqlite-pager private_memory -- --nocapture`
    passed in the temporary worktree.
  - Baseline:
    `tests/artifacts/perf/private-memory-commit-base-purplecoast-20260505T1120Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/private-memory-commit-candidate-purplecoast-20260505T1120Z/report.json`.
- Result: rejected and not applied to the shared worktree. The insert ratio
  summary looked better (`avg_ratio 2.283x -> 2.097x`, weighted score
  `1.6279 -> 1.4773`), but the absolute FrankenSQLite medians were worse:
  geomean time ratio `1.107x`, average time ratio `1.127x`, with `17/25`
  insert rows slower. Notable regressions included small_3col 10K autocommit
  `13.09 ms -> 21.48 ms`, small_3col 1K autocommit
  `1.52 ms -> 2.24 ms`, 100-row batched `218.9 us -> 323.6 us`, and
  large_10col 10K single transaction `42.18 ms -> 44.28 ms`.
- Do not retry private `:memory:` WAL bypass as a standalone pager shortcut.
  Revisit only with a same-window proof that improves absolute FrankenSQLite
  medians and the insert-section score; ratio-only gains are suspect because
  the C SQLite denominator can move enough to hide FrankenSQLite regressions.

## 2026-05-05 - PageData shared-pair quick-balance handoff

Scope: full `comprehensive-bench --filter insert` after the exact-divider
quick-balance win, targeting the full-page clone in
`crates/fsqlite-btree/src/balance.rs::balance_quick_known_divider_rowid`.

- Candidate shape: add `PageData::into_shared_pair()` in
  `crates/fsqlite-types/src/lib.rs` and use it to move the freshly split right
  sibling page into one `Arc<[u8]>`, handing one shared handle to the writer and
  one shared handle back to the rightmost-leaf cache.
- Evidence: first run
  `tests/artifacts/perf/insert-pagedata-shared-pair-cyangorge-20260505T121337Z/`;
  rerun
  `tests/artifacts/perf/insert-pagedata-shared-pair-rerun-cyangorge-20260505T121651Z/`;
  baseline
  `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/`.
- Result: rejected and reverted. The aggregate ratio moved in the right
  direction on the rerun (`geomean_ratio 2.3519x -> 2.1634x`, weighted score
  `1.7141 -> 1.6914`), but the split-heavy absolute FrankenSQLite medians
  regressed: `large_10col` single-transaction 10K
  `34.756 ms -> 38.651 ms`, and 100K `415.902 ms -> 444.772 ms`.
  The root cause is representation semantics: the existing `PageData::clone()`
  path pays one snapshot clone but keeps the cursor's new rightmost page backed
  by owned mutable bytes; the shared-pair variant made the cursor cache shared
  too, so the next append to that page pays copy-on-write.
- Do not retry by making both split-page handles shared. A future version would
  need a writer handoff that preserves an owned mutable page for the cursor, or
  a different rightmost-cache design, and must improve the large-row absolute
  medians in the same insert matrix.

## 2026-05-05 - Direct INSERT rowid-alias double-eval skip

Scope: `comprehensive-bench --quick --filter insert` after the current
`237261d2` full quick matrix showed the remaining biggest ratios clustered in
write-heavy insert rows. The candidate targeted the compiled direct-insert row
builder in `crates/fsqlite-core/src/connection.rs`.

- Candidate shape: after `eval_prepared_direct_simple_insert_explicit_rowid_only`
  had already evaluated the INTEGER PRIMARY KEY alias expression for append
  routing, skip re-evaluating the same compiled rowid/IPK expression in the
  row-build loop and push the storage `NULL` placeholder directly.
- Evidence: baseline insert profile
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/`;
  candidate
  `tests/artifacts/perf/insert-rowid-alias-skip-cyangorge-20260505T123625Z/`.
  Focused tests passed before the A/B:
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
  and
  `cargo test -p fsqlite-core test_prepared_direct_insert_without_change_tracking_skips_tls_sync -- --nocapture`.
- Result: rejected and reverted. The insert section regressed
  (`geomean_ratio 2.3623x -> 2.4502x`, weighted score
  `1.6991 -> 1.7605`, p99 `4.1407x -> 4.3519x`). The targeted
  `large_10col` single-transaction 10K median improved only slightly
  (`36.165 ms -> 35.335 ms`) while the record-size `large_10col` 10K row
  regressed (`37.056 ms -> 37.477 ms`) and multiple smaller insert rows
  worsened.
- Do not retry rowid-alias double-eval skipping as a standalone direct-insert
  micro-optimization. The skipped expression is too cheap relative to row text
  construction, B-tree work, and commit publication, and the codegen perturbation
  did not move the matrix.

## 2026-05-05 - Direct INSERT concat owned-text move

Scope: `comprehensive-bench --quick --filter insert` with
`FSQLITE_BENCH_PROFILE_INSERT=1`, targeting the direct-simple INSERT concat
row builder in `crates/fsqlite-core/src/connection.rs` after profiles showed
large-row `row_build_ns` around 5-6 ms for 10K-row large-record inserts.

- Candidate shape: keep inline-size concat strings on the existing borrowed
  `SmallText::new` path, but for longer concat results move the reusable
  `String` scratch into `SmallText::from_string` instead of copying
  `text_scratch.as_str()` into a second heap string.
- Evidence: same-window baseline
  `tests/artifacts/perf/insert-concat-owned-text-baseline-cyangorge-20260505T124529Z/`;
  candidate
  `tests/artifacts/perf/insert-concat-owned-text-cyangorge-20260505T125310Z/`.
  Focused proof tests passed before the A/B:
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
  and
  `cargo test -p fsqlite-core test_prepared_direct_insert_without_change_tracking_skips_tls_sync -- --nocapture`.
- Result: rejected and reverted. Insert geomean regressed
  `2.2471x -> 2.5245x`, weighted score regressed `1.6366 -> 1.7467`,
  and p99 regressed `3.7572x -> 4.4258x`. The target large rows also
  regressed in absolute FrankenSQLite medians:
  `large_10col` single-transaction 10K `35.292 ms -> 43.055 ms`, and
  record-size `large_10col` 10K `36.379 ms -> 41.902 ms`.
- Do not retry concat owned-string moving as a standalone direct INSERT row
  builder optimization. The root cause is allocator locality: moving the
  scratch avoids one copy but destroys scratch-capacity reuse, forcing the hot
  concat builder to reallocate repeatedly. Future row-build work should avoid
  materializing transient `SqliteValue::Text` for lazy `:memory:` inserts or
  serialize concat output directly into a record/page destination with a
  same-window insert matrix win.

## 2026-05-05 - Quotient-filter empty-map maintenance skip

Scope: direct INSERT per-row bookkeeping in
`crates/fsqlite-core/src/connection.rs`, after insert profiles showed
substantial execute-body time not fully covered by row-build, serialization,
B-tree, and commit counters. The candidate targeted `qf_record_insert` /
`qf_record_delete`, which are called after successful direct-simple INSERT and
DELETE maintenance.

- Candidate shape: return early when `self.quotient_filters.borrow().is_empty()`
  before taking the existing mutable borrow and attempting a root-page lookup.
  The intended fast path was benchmark-style INSERT workloads where no
  quotient filter has been seeded yet, making QF maintenance a logical no-op.
- Evidence: correctness gate failed before benchmarking:
  `cargo test -p fsqlite-core quotient_filter -- --nocapture`. Artifact note:
  `tests/artifacts/perf/insert-qf-empty-skip-cyangorge-20260505T1256Z/summary.md`.
- Result: rejected and reverted before A/B measurement. Two existing tests
  failed: `test_quotient_filter_short_circuits_absent_rowids_on_delete`
  reported `expected >= 90 QF short-circuits, got 0`, and
  `test_quotient_filter_delete_then_redelete_short_circuits` reported that the
  second delete of a removed rowid did not short-circuit.
- Do not retry an empty-map early return in QF maintenance without first
  reworking the lazy seed lifecycle. The empty-map state is not merely an
  inert "disabled" state; it can be part of the path that lets later DELETE /
  UPDATE consultation seed and maintain the filter correctly.

## 2026-05-05 - Retained autocommit count-sum explicit transaction skip

Scope: direct-simple INSERT per-row bookkeeping in
`crates/fsqlite-core/src/connection.rs`, after insert profiles showed large
unaccounted execute-body time beyond row-build, serialization, B-tree insert,
and commit counters. The candidate targeted
`retained_autocommit_count_sum_cache_note_insert`, which runs after successful
direct-simple INSERT.

- Candidate shape: return early from
  `retained_autocommit_count_sum_cache_note_insert` when
  `self.in_transaction.get()` is true. Explicit `BEGIN..COMMIT` insert
  workloads cannot seed the retained autocommit count/sum cache because
  `maybe_seed_retained_autocommit_count_sum_cache_from_clean_memdb` already
  returns inside a transaction, so the candidate tried to avoid one per-row
  cache path.
- Evidence: same-window baseline
  `tests/artifacts/perf/insert-concat-owned-text-baseline-cyangorge-20260505T124529Z/`;
  candidate
  `tests/artifacts/perf/insert-retained-cache-explicit-skip-cyangorge-20260505T130650Z/`.
  Focused correctness/build gates passed before the A/B:
  `cargo fmt --check`,
  `cargo test -p fsqlite-core retained_autocommit_count_sum_cache -- --nocapture`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`,
  and
  `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Result: rejected and reverted. Insert geomean regressed
  `2.2471x -> 2.4574x`, weighted score regressed `1.6366 -> 1.7698`,
  and p99 regressed `3.7572x -> 4.0913x`. The target large rows also
  regressed in absolute FrankenSQLite medians:
  `large_10col` single-transaction 10K `35.292 ms -> 36.626 ms`, and
  record-size `large_10col` 10K `36.379 ms -> 36.733 ms`.
- Do not retry explicit-transaction skipping of retained-autocommit count/sum
  cache maintenance as a standalone direct INSERT optimization. The cache path
  is logically redundant for this workload, but the branch/codegen perturbation
  was not free and the benchmark matrix moved the wrong way.

## 2026-05-05 - Agent Mail CASS/git addenda: remaining no-retry shapes

Scope: patch-ready peer handoff from the last-60-day CASS/git negative-result
expansion while this agent held `docs/progress/perf-negative-results.md`.
Direct `/data/projects/frankensqlite` CASS workspace searches were sparse, so
the useful leads came from `cass search "frankensqlite <term>" --days 60`,
archived Gemini FrankenSQLite sessions, preserved artifacts, and recent revert
commits. Entries already present in this ledger were not duplicated.

- Broad and parent-only structural preclaim: rejected and reverted on the
  flagship `commutative_inserts_disjoint_keys / frankensqlite / c8` row. The
  broad shape preclaimed structural pages before split/rebalance writes via
  `crates/fsqlite-btree/src/cursor.rs` plus VDBE preclaim/rollback plumbing;
  the parent-only narrowing was even worse. Evidence includes
  `artifacts/perf/20260314_direct_handle_owned_fastpath_pass3/disjoint_c8_release_perf_both.jsonl`,
  `artifacts/perf/20260314_direct_handle_owned_fastpath_v2/disjoint_c8_release_perf_both.jsonl`,
  `artifacts/perf/20260314_structural_preclaim/disjoint_c8_release_perf_both.jsonl`,
  `artifacts/perf/20260314_parent_preclaim/disjoint_c8_release_perf_both.jsonl`,
  and `docs/planning/STATE_OF_THE_CODEBASE_AND_NEXT_STEPS.md`. Do not retry
  earlier deterministic claiming of shared B-tree structure as the concurrency
  fix; it lengthened the convoy and widened the effective choke point. Future
  work must reduce shared structural work, shorten hold duration, or change
  physical layout, and rerun the full focused c1/c4/c8 family.
- Quotient-filter build-on-first-consult for direct UPDATE/DELETE: rejected as
  a severe benchmark regression. The lazy build-on-first-consult path scanned
  the full table at the first DELETE/UPDATE after `Connection::open`; commit
  `4ea55010` records `update-deletethroughput__100-rows-delete-5-rows`
  regressing by about `369x` because a roughly `30 ms` scan was added to a
  roughly `0.1 ms` delete. Do not lazily build rowid membership filters on
  first DML consult for existing tables. Retry only with an explicit activation
  policy where build cost is known-zero or amortized outside the target
  operation, and prove the UPDATE/DELETE matrix moves.
- Mechanical `SqliteValue` Arc conversion via Python/cargo-fix traversal: do
  not repeat it. March CASS shows the text/blob Arc idea was attempted as a
  broad conversion across tests, property macros, and record/value helpers; it
  caused serde/type mismatches, mangled `record.rs` / `value.rs` patterns and
  test assertions, and required reverting to `String` / `Vec<u8>`. This is the
  process-specific variant of the broader Arc no-retry rule: retry only with a
  designed serde/API migration and narrow hand-edited proof, never as a
  mechanical repo sweep.
- Read-heavy `query_row_with_params()` wrapper swap: rejected. The
  `mt_read_bench` pass changed the FSQLite side from `query_with_params()` to
  `query_row_with_params()` and made the remote matrix worse (`0.05x`,
  `0.06x`, `0.07x`, `0.25x`), then reverted it before closeout. Evidence:
  `tests/artifacts/perf/read-heavy-20260430T021702Z/RESULT.md`. Do not retry a
  query/query_row wrapper substitution; the documented next lever is
  file-backed prepared MemDB direct lookup after read-state refresh.
- `concat_ws` pre-sizing scan: rejected as slower than the accepted direct
  append path. The direct append candidate measured `24,453,767 ns`, while the
  pre-sizing pass measured `34,885,096 ns` because the extra scan outweighed
  saved growth for the 24-text-argument benchmark. Evidence:
  `tests/artifacts/perf/20260428T2100Z-icybluff-concat-ws-direct/RESULT.md`.
  Keep the direct-append implementation; do not add a pre-size scan unless a
  new workload has much larger output growth and proves the scan pays for
  itself.
- Mixed-OLTP omitted rowid-alias projection remapping: rejected. The
  double-parse version averaged only about `0.6%` absolute FrankenSQLite
  improvement and the one-pass rewrite regressed repeat measurements, so both
  were rolled back. Evidence:
  `tests/artifacts/perf/20260425T1921Z-azurepine-alias-projection-fastpath/summary.md`.
  Do not retry IPK-alias projection remapping as an isolated COUNT/SUM lever
  unless the mixed matrix moves beyond the keep threshold.
- Manual integer decode assembly in `decode_big_endian_signed`: rejected.
  Absolute FrankenSQLite movement stayed under `1%` and normalized F/C ratio
  worsened despite passing direct sign-extension and integer-boundary proofs.
  Evidence:
  `tests/artifacts/perf/20260425T1921Z-azurepine-alias-projection-fastpath/summary.md`.
  Do not replace the current integer decoder with hand assembly for scalar
  microbench reasons alone.
- Rowid-only local leaf fast path for retained dirty-overlay range counting:
  rejected. F median improved, but the two-run average was only about `2.2%`
  faster than the accepted local-leaf payload-prefix baseline and stayed below
  the keep threshold; the patch was rolled back. Evidence:
  `tests/artifacts/perf/20260425T1921Z-azurepine-alias-projection-fastpath/summary.md`.
  Do not re-add a rowid-only local-leaf branch unless the retained range-count
  row is again a top matrix gap and clears the threshold.
- `xxh3_64` to `crc32c` for `page_mutation_counter`: rejected on this host.
  The April profiling handoff records the T5a experiment as reverted because
  `crc32c` was `28%` slower for 4 KiB inputs. Evidence:
  `tests/artifacts/perf/profiling-handoff-20260423T155542Z/campaign-summary.md`
  and `tests/artifacts/perf/bd-cnk5d-2t-cliff-verify-20260424/summary.md`.
  Do not swap hash functions because CRC32C sounds hardware-accelerated;
  require a same-host checksum/profile proof and a matrix win.
- `PublishedPagerState::new` / connection-open cost as a standalone target:
  false lead for production-style workloads. The profiling handoff marks it as
  connection-open cost visible in microbenches that open fresh connections, not
  an operation-count cost for long-lived connections. Evidence:
  `tests/artifacts/perf/profiling-handoff-20260423T155542Z/hypothesis-ledger.md`.
  Do not spend a perf pass optimizing this in isolation unless a connection
  pool or open-heavy workload is the explicit benchmark target.

## 2026-05-05 - Pager EOF page-lease batch size 8 -> 32

- Target: INSERT throughput, especially large single-transaction rows that
  allocate about 2K pages and show `page_pool_misses=2013` plus multi-ms B-tree
  quick-balance/commit time.
- Touched during rejected candidate: `crates/fsqlite-pager/src/pager.rs`
  (`PAGE_LEASE_BATCH_SIZE`). Reverted to `8` after measurement.
- Candidate shape: increase `PAGE_LEASE_BATCH_SIZE` from `8` to `32` so
  concurrent transactions pre-allocate follow-on EOF pages in larger batches,
  aiming to reduce repeated `inner` mutex acquisitions during right-edge B-tree
  splits.
- Evidence artifacts:
  `tests/artifacts/perf/page-lease-8-baseline-purplecoast-20260505T1316Z/report.json`
  and
  `tests/artifacts/perf/page-lease-32-candidate-purplecoast-20260505T1322Z/report.json`.
- Result: rejected and reverted. The focused insert matrix worsened overall
  average ratio from `2.36x` to `2.56x`. Primary large-row medians did not
  improve: `large_10col` single transaction FSQLite moved `37.57 ms` to
  `38.27 ms`, and record-size `large_10col` moved `36.99 ms` to `42.37 ms`.
  Medium 10K single transaction also worsened `14.28 ms` to `15.88 ms`; small
  10K worsened `7.25 ms` to `7.96 ms`.
- Do not retry as a standalone larger EOF lease batch. Reconsider only with a
  page-allocation profile showing `TransactionHandle::allocate_page`
  inner-lock acquisition dominating and an adaptive policy that preserves or
  improves the full insert matrix, especially the large record-size row.

## 2026-05-05 - External quick-balance owned page handoff

- Target: INSERT throughput rows that split the rightmost leaf through
  `try_quick_balance_on_external_rightmost_leaf_hint`, especially 10K
  `large_10col` single-transaction and record-size rows.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`.
  The code was reverted after measurement.
- Candidate shape: on the external retained-hint quick-balance success path,
  move `result.new_page_data` directly into `hint.page_data` and clear
  `rightmost_leaf_cache` instead of cloning the page into the hint and storing
  another owned copy in the cursor-local cache.
- Correctness smoke:
  `cargo test -p fsqlite-btree test_table_try_append_cached_rightmost_leaf_hint -- --nocapture`
  passed (`4` tests).
- Evidence artifacts:
  `tests/artifacts/perf/qb-owned-handoff-baseline-dirtyconn-purplecoast-20260505T132841Z/report.json`,
  `tests/artifacts/perf/qb-owned-handoff-candidate-purplecoast-20260505T132443Z/report.json`,
  and
  `tests/artifacts/perf/qb-owned-handoff-candidate-repeat-purplecoast-20260505T133407Z/report.json`.
  A peer dirty-tree check reached the same disposition in
  `tests/artifacts/perf/insert-external-qb-hint-current-dirty-cyangorge-20260505T1333Z/summary.md`.
- Result: rejected and reverted. The primary weighted score looked better in
  the local paired runs (`1.8386` baseline to `1.7808` / `1.7728` candidate),
  but this was not a full-workload win: geomean ratio worsened on both
  candidates (`2.5061x` to `2.5690x` / `2.6859x`), write-bulk worsened on the
  repeat (`2.8074x` to `2.9619x`), and the main 10K `large_10col`
  single-transaction FSQLite median worsened `37.61 ms` to `39.12 ms` and then
  `42.22 ms`. Record-size tiny/small/medium rows consistently regressed
  (`4.50/6.78/10.68 ms` baseline to `5.36/8.00/12.04 ms`, then
  `6.03/8.01/12.75 ms`). The only large record-size improvement was unstable
  (`44.89 ms` baseline to `37.97 ms`, then `43.03 ms`).
- Do not retry this exact "move page to external hint and clear internal
  cache" handoff. Reconsider only with a different rightmost-cache design that
  avoids the page clone while preserving the useful cache state, and require an
  interleaved A/B that improves the full insert matrix without regressing the
  small/medium record-size rows or the 10K large single-transaction row.

## 2026-05-05 - Direct INSERT integer placeholder text cache

- Target: direct-simple INSERT concat row building after the insert profile
  showed multi-ms row-build cost on 10K medium/large rows. The candidate was
  tested in the isolated worktree
  `/data/tmp/frankensqlite-cyangorge-paramtext-cache-20260505T1340` so the
  shared main worktree and peer source edits were not disturbed.
- Candidate shape: add a stack-local cache for integer bind placeholder decimal
  text during one direct INSERT row build, aiming to avoid repeated `itoa`
  formatting for repeated concat references such as `?1`.
- Correctness smoke passed in the isolated worktree:
  `cargo fmt --check`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`,
  `cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`,
  and `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifacts:
  `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`
  and
  `tests/artifacts/perf/insert-param-text-cache-cyangorge-20260505T1347Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-param-text-cache-cyangorge-20260505T1347Z/summary.md`.
- Result: rejected and not applied to main. The focused insert matrix worsened:
  geomean F/C ratio `2.3832x` to `2.5280x`, weighted score `1.6578` to
  `1.7978`, write-bulk geomean `2.5538x` to `2.6975x`, and write-single
  geomean `1.4354x` to `1.5703x`. Target large rows did not improve:
  single-transaction `large_10col` 10K moved `37.5866 ms` to `37.7624 ms`,
  and record-size `large_10col` moved `39.4682 ms` to `41.3979 ms`.
- Do not retry per-row integer placeholder text caching as a standalone
  row-build optimization. Reconsider only with a direct serialization design
  that avoids transient text materialization rather than caching its decimal
  representation, and require a full insert-matrix win.

## 2026-05-05 - Dirty WAL prepared-frame direct publication snapshot

- Target: INSERT commit/publish cost on large single-transaction rows, where
  profiles still show multi-ms `commit_roundtrip_ns`. The measured source diff
  was peer-owned dirty work in `crates/fsqlite-core/src/wal_adapter.rs`; this
  entry records an independent dirty-tree A/B, not a source change landed by
  CyanGorge.
- Candidate shape: for prepared frame batches with a known commit frame, publish
  the WAL visibility snapshot directly from `prepared.frame_metas` instead of
  first recording those frame entries in `pending_publication_frames`.
- Correctness smoke:
  `cargo test -p fsqlite-core --lib append -- --nocapture` passed
  (`17` tests). A broader exploratory
  `cargo test -p fsqlite-core append -- --nocapture` run passed the WAL adapter
  append tests but failed
  `test_v2_plain_execute_sequential_inserts_keep_append_path_hot_across_statements`,
  so that integration failure must be resolved or shown unrelated before
  landing.
- Evidence artifacts:
  `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`
  and
  `tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/summary.md`.
- Result: mixed and not a keep as-is. Large FSQLite medians improved
  (`large_10col` single transaction `37.5866 ms` to `35.1876 ms`,
  record-size `large_10col` `39.4682 ms` to `34.7089 ms`), but the insert
  matrix did not clear the keep gate: geomean F/C ratio worsened slightly
  `2.3832x` to `2.3890x`, weighted score worsened `1.6578` to `1.7359`,
  and write-single geomean worsened `1.4354x` to `1.5293x`.
- Do not land this exact direct-publish dirty diff from this evidence alone.
  Retry only with an interleaved clean/candidate A/B that preserves the
  large-row improvement, restores the weighted score/write-single rows, and
  explains or fixes the broader append-filter failure.

## 2026-05-05 - Thresholded WAL prepared-frame direct publication

- Target: same large INSERT commit/publish cost as the dirty direct-publication
  check, but with a frame-count threshold intended to keep small/write-single
  commits on the existing path.
- Touched during rejected isolated candidate:
  `crates/fsqlite-core/src/wal_adapter.rs` in temporary worktree
  `/data/tmp/frankensqlite-cyangorge-wal-threshold-20260505T1406`; the shared
  source file was reserved by PurpleCoast and was not edited.
- Candidate shape: factor WAL commit snapshot publication over generic frame
  entries, then use direct publication from `prepared.frame_metas` only when
  `prepared.frame_count() >= 128`. A new 128-frame unit test asserted the large
  direct path publishes all pages and leaves no pending publication entries.
- Correctness smoke:
  `cargo fmt`,
  `cargo test -p fsqlite-core --lib append -- --nocapture` passed
  (`18` tests), and
  `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
  passed in the isolated worktree.
- Evidence artifacts:
  `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`,
  `tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/report.json`,
  and
  `tests/artifacts/perf/insert-wal-publish-threshold-cyangorge-20260505T1406Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-wal-publish-threshold-cyangorge-20260505T1406Z/summary.md`.
- Result: rejected. The thresholded variant was worse than both clean baseline
  and the full dirty direct-publication variant: geomean F/C ratio `2.3832x`
  baseline / `2.3890x` full-direct / `2.5341x` threshold, weighted score
  `1.6578` / `1.7359` / `1.8890`, and write-single geomean `1.4354x` /
  `1.5293x` / `1.6811x`. It also failed to preserve the full direct large-row
  win: record-size `large_10col` F median was `39.4682 ms` baseline,
  `34.7089 ms` full-direct, and `38.2606 ms` threshold.
- Do not retry a simple frame-count threshold around WAL prepared-frame direct
  publication. First prove why the full direct path improved large rows, then
  design a narrower change that does not disturb write-single or B-tree timing.

## 2026-05-05 - CASS strict project-folder follow-up

Scope: user-requested CASS pass restricted to the last 60 days. Direct
`--workspace /data/projects/frankensqlite` searches for `rejected`,
`reverted`, `slower`, `didn't help`, `abandoned`, and the misspelling
`abandones` found only a sparse 2026-03-07 direct-workspace slice and no direct
negative-term hits. To avoid treating that as an empty history, the follow-up
used CASS workspace aliases whose source paths clearly map to this repo,
especially `/home/ubuntu/.gemini/tmp/frankensqlite`, then cross-checked leads
against preserved perf artifacts before recording them here.

- Session-shared page-1 synthetic hint flag: rejected after the target
  `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` profile stack
  dropped but `perf-update-delete 10000 100 both` stayed inside noise. Baseline
  mean was `1.206 s +/- 0.021 s`; candidate v3 mean was
  `1.204 s +/- 0.025 s` (`1.00 +/- 0.03` faster). Evidence:
  `tests/artifacts/perf/20260428T2230Z-sapphirecrane-page1-synthetic-flag/RESULT-page1-synthetic-flag.md`.
  Do not add session-shared page-1 hint state in `Connection` /
  `SharedTxnPageIo` merely because the narrow stack disappears; require a
  measurable update/delete matrix win.
- Unguarded rowid-count helper for larger right tables: this reinforces the
  existing rowid-count guardrail with a clean local A/B. Removing
  `ROWID_COUNT_SMALL_RIGHT_ROW_LIMIT` improved only the 100-order HAVING row
  (`0.2168 ms` to `0.2113 ms`) but regressed the 1000-order HAVING row
  materially (`1.2285 ms` to `1.6221 ms`) and did not improve the 10000-order
  row (`10.6338 ms` to `10.7713 ms`). Evidence:
  `tests/artifacts/perf/join-rowid-count-large-candidate-purplecoast-20260504T2045Z/summary.md`.
  Do not remove the rowid-count right-table guard without a close join-section
  A/B that improves all affected row counts or the section score.
- March raw-`bench_insert` hash-swap/cache experiments are stale evidence, not
  a keep/retry basis. CASS shows attempts to justify `foldhash` swaps in SQL
  cache, cursor/hash maps, pager `PageCache`, and `MemPageStore` from the old
  raw-string `bench_insert` profile while repeated compile churn and background
  edits prevented a stable current-matrix proof. This reinforces the existing
  stale-benchmark rule: retry hash-function or dense-index storage changes only
  from a current prepared-statement matrix/profile, not from old raw SQL-string
  cache-thrash sessions.

CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 84 -C 60`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json -n 120 -C 40`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-628c8b17.json -n 90 -C 35`

## 2026-05-05 - Direct INSERT row-value text pooling

- Target: prepared direct INSERT row-build cost on medium/large concat-heavy
  rows after the current insert profile showed `row_build_ns` around
  `5.96 ms` on both large 10K single-transaction rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after the benchmark.
- Candidate shape: return heap-backed `SqliteValue::Text` row-scratch values
  to the existing `fsqlite_types::value` TLS pool when lazy private-memory
  direct inserts clear `mem_row_values`, then build concat-chain text results
  from a pooled `SmallText` slot via `SmallText::overwrite`.
- Correctness/build smoke passed before the A/B:
  `cargo fmt --check`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_returns_concat_text_to_value_pool -- --nocapture`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`,
  and `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifacts:
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`
  and
  `tests/artifacts/perf/insert-row-text-pool-cyangorge-20260505T1434Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-row-text-pool-cyangorge-20260505T1434Z/summary.md`.
- Result: rejected and reverted. Insert avg/geomean ratios improved
  (`2.4610x -> 2.3595x`, `2.3623x -> 2.2890x`), but the primary weighted
  insert score regressed `1.6991 -> 1.7329` and write-single geomean regressed
  `1.4908x -> 1.5517x`. Important absolute FrankenSQLite medians worsened:
  `small_3col` 1K single transaction `0.8055 ms -> 0.9613 ms`, `small_3col`
  10K single transaction `6.8949 ms -> 7.7481 ms`, `medium_6col` 10K
  `13.6661 ms -> 14.6216 ms`, `large_10col` 10K `36.1651 ms -> 36.7869 ms`,
  and record-size `large_10col` `37.0559 ms -> 37.6541 ms`.
- Do not retry direct INSERT row-value pooling / pooled `SmallText::overwrite`
  as a standalone row-build optimization. The profile counters showed the
  root hypothesis failed on the target large rows: row-build time got worse
  (`large_10col` single transaction `5.958 ms -> 7.404 ms`, record-size
  `large_10col` `5.973 ms -> 6.722 ms`), so TLS pool traffic cost more than
  it saved.

## 2026-05-05 - Benchmark-only journal_mode=MEMORY switch

- Target: private `:memory:` benchmark write gap, especially large INSERT
  rows. The motivating observation was that C SQLite reports and keeps
  `journal_mode=memory` for `:memory:` even after `PRAGMA journal_mode=WAL`,
  while FrankenSQLite honors WAL for private in-memory databases.
- Touched during rejected candidate:
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`; source was reverted
  after measurement.
- Candidate shape: change the benchmark pragma setup from
  `PRAGMA journal_mode = WAL` to `PRAGMA journal_mode = MEMORY` for both
  C SQLite and FrankenSQLite.
- Evidence artifacts:
  `tests/artifacts/perf/insert-journal-memory-candidate-purplecoast-20260505T1450Z/report.json`
  and
  `tests/artifacts/perf/full-quick-journal-memory-candidate-purplecoast-20260505T1515Z/report.json`.
  Summaries:
  `tests/artifacts/perf/insert-journal-memory-candidate-purplecoast-20260505T1450Z/summary.md`
  and
  `tests/artifacts/perf/full-quick-journal-memory-candidate-purplecoast-20260505T1515Z/summary.md`.
- Insert-only result looked tempting: weighted insert score improved
  `1.6991 -> 1.6703`, geomean ratio improved `2.3623x -> 2.2924x`,
  write_bulk geomean improved `2.5153x -> 2.4349x`, and absolute large-row
  FrankenSQLite medians improved (`large_10col` 10K single transaction
  `36.165 ms -> 33.412 ms`, record-size `large_10col` 10K
  `37.056 ms -> 34.171 ms`).
- Full quick matrix rejected it: weighted score worsened
  `0.5658 -> 0.5808`, avg/geomean ratios worsened `1.0270x -> 1.0691x` and
  `0.4467x -> 0.4596x`, write_bulk geomean worsened `2.3562x -> 2.4735x`,
  write_single worsened `2.0563x -> 2.1667x`, and concurrent writers worsened
  `1.1514x -> 1.1830x`.
- Do not retry the benchmark-only `journal_mode=MEMORY` switch as a standalone
  fairness/performance correction. It is only worth revisiting as part of a
  broader benchmark policy change that improves or preserves the full
  end-to-end matrix, not merely the insert-only rows.

## 2026-05-05 - insert_page_sorted append/equal fast path

- Target: sequential INSERT write-set staging in the pager, where page numbers
  are often appended in sorted order.
- Touched during rejected isolated candidate:
  `crates/fsqlite-pager/src/pager.rs` in clean worktree
  `/data/tmp/frankensqlite-purplecoast-clean-20260505T1458`; shared source was
  not edited or staged by this measurement.
- Candidate shape: check `pages.last()` in `insert_page_sorted` and return
  immediately for monotonic append (`last < page_no`) or duplicate-last
  (`last == page_no`) before falling back to the existing binary search and
  insertion path.
- Evidence artifact:
  `tests/artifacts/perf/insert-page-sorted-append-candidate-purplecoast-20260505T1504Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-page-sorted-append-candidate-purplecoast-20260505T1504Z/summary.md`.
- Result: rejected. Avg/geomean ratios improved slightly
  (`2.4610x -> 2.4231x`, `2.3623x -> 2.3470x`) and write_bulk geomean improved
  (`2.5153x -> 2.4909x`), but the primary weighted insert score regressed
  `1.6991 -> 1.7171` and write-single geomean regressed
  `1.4908x -> 1.5168x`.
- Do not retry the simple `insert_page_sorted` last-page append/equal branch as
  a standalone optimization. The branch is cheap and plausible, but current
  end-to-end insert evidence says it is not a keep.

## 2026-05-05 - WAL publication page-index Arc::make_mut hoist

- Target: large INSERT commit publication overhead in
  `WalBackendAdapter::publish_pending_commit_snapshot`.
- Touched during rejected candidate: `crates/fsqlite-core/src/wal_adapter.rs`;
  source was reverted after measurement.
- Candidate shape: hoist `Arc::make_mut(&mut page_index)` out of the
  per-frame loop so a commit that publishes thousands of frames only performs
  the mutable Arc access once.
- Correctness/build smoke passed:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-makemut-target cargo test -p fsqlite-core --lib append -- --nocapture`
  (`17` tests) and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-makemut-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifact:
  `tests/artifacts/perf/insert-wal-page-index-makemut-purplecoast-20260505T1513Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-wal-page-index-makemut-purplecoast-20260505T1513Z/summary.md`.
- Result: rejected. Insert weighted score regressed `1.6991 -> 1.8022`,
  avg/geomean ratios regressed `2.4610x -> 2.5586x` and
  `2.3623x -> 2.4753x`, write_bulk regressed `2.5153x -> 2.6295x`, and
  write_single regressed `1.4908x -> 1.5889x`.
- Do not retry this simple `Arc::make_mut` hoist as a standalone WAL
  publication optimization. The branch looked mechanically cheaper, but the
  current end-to-end insert matrix rejected it.

## 2026-05-05 - Direct INSERT precomputed column affinities

- Target: direct-simple INSERT row value handling in
  `crates/fsqlite-core/src/connection.rs`, after perf showed visible time in
  `push_prepared_direct_simple_insert_value` / `SqliteValue::apply_affinity`
  on the insert matrix.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after measurement.
- Candidate shape: add `column_affinities: Vec<TypeAffinity>` to
  `PreparedDirectSimpleInsert`, compute it once during
  `prepared_direct_simple_insert_plan`, and pass the precomputed enum to
  `push_prepared_direct_simple_insert_value` instead of calling
  `type_affinity_for_direct_insert(column.affinity)` for every inserted column.
- Correctness smoke passed:
  `cargo fmt --check` and
  `env CARGO_TARGET_DIR=/data/tmp/cargo-target cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
  (`28` matching tests).
- Evidence artifacts:
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/baseline-report.json`
  and
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/candidate-report.json`.
  Summary:
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/summary.md`.
- Result: rejected. The primary weighted insert score regressed
  `1.5606 -> 1.8360`, avg/geomean ratios regressed
  `2.3295x -> 2.5739x` and `2.2311x -> 2.4638x`, write_bulk geomean
  regressed `2.3883x -> 2.6058x`, and write_single geomean regressed
  `1.3542x -> 1.6338x`. The target large-row row-build counters did not
  improve reliably: `large_10col` single txn row_build_ns was essentially flat
  (`6114165 -> 6115810`), while record-size `large_10col` worsened
  (`5951537 -> 6813546`).
- Do not retry precomputing direct INSERT column affinity metadata as a
  standalone micro-optimization. The per-row char-to-affinity match is not the
  bottleneck; future affinity work should remove or fuse value coercion itself
  and must improve the same-window insert matrix.

## 2026-05-05 - WAL checksum one-chunk header transform

- Target: `WalChecksumTransform::for_wal_frame` self-time under large INSERT
  WAL frame preparation.
- Touched during rejected candidate: `crates/fsqlite-wal/src/checksum.rs`;
  source was reverted after measurement.
- Candidate shape: replace the generic
  `WalChecksumTransform::from_aligned_bytes(&frame[..8], ...)` call for the
  8-byte WAL frame header prefix with the closed-form affine transform for
  exactly one checksum chunk. The page payload transform stayed on the generic
  path.
- Correctness smoke passed:
  `cargo fmt --check` and
  `env CARGO_TARGET_DIR=/data/tmp/cargo-target cargo test -p fsqlite-wal checksum_transform -- --nocapture`
  (`2` matching tests). The first release-perf build attempt in the shared
  `/data/tmp/cargo-target` failed with a missing bytecode file, so the
  candidate benchmark was built in the unique target dir
  `/data/tmp/frankensqlite-cyangorge-walchk-target`.
- Evidence artifacts:
  baseline
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/baseline-report.json`
  and candidate
  `tests/artifacts/perf/wal-checksum-header-transform-cyangorge-20260505T1535Z/candidate-report.json`.
  Summary:
  `tests/artifacts/perf/wal-checksum-header-transform-cyangorge-20260505T1535Z/summary.md`.
- Result: rejected. The primary weighted insert score regressed
  `1.5606 -> 1.7049`, avg/geomean ratios regressed
  `2.3295x -> 2.4746x` and `2.2311x -> 2.3800x`, write_bulk geomean
  regressed `2.3883x -> 2.5361x`, and write_single geomean regressed
  `1.3542x -> 1.4935x`. Several absolute FSQLite 10K medians improved, but
  the ratio-weighted matrix and category scores failed the keep gate.
- Do not retry a special one-chunk header transform inside
  `WalChecksumTransform::for_wal_frame` as a standalone micro-optimization.
  Future WAL checksum work should reduce the payload transform or prepared-frame
  pipeline cost and must improve the full insert matrix.

## 2026-05-05 - Defaulting INSERT commits to parallel WAL auto lane staging

- Target: default conservative WAL group-commit path for `:memory:` INSERT
  workloads, after detailed commit profiling split phase B into batch-build,
  flusher frame-prep, and append-call costs.
- Candidate shape: use `FSQLITE_PARALLEL_WAL_MODE=auto` as the behavior proxy
  for changing the default from conservative WAL staging to lane-local prepared
  WAL staging. No source behavior change was kept.
- Evidence artifacts:
  - Default/conservative current-code run:
    `tests/artifacts/perf/insert-commit-phase-split-gated-cyangorge-20260505T2058Z/report.json`
    and
    `tests/artifacts/perf/insert-commit-phase-split-gated-cyangorge-20260505T2058Z/summary.md`.
  - Auto-lane candidate run:
    `tests/artifacts/perf/insert-commit-phase-split-auto-cyangorge-20260505T2050Z/report.json`
    and
    `tests/artifacts/perf/insert-commit-phase-split-auto-cyangorge-20260505T2050Z/summary.md`.
- Result: rejected. Auto mode regressed the insert matrix average ratio
  `2.51x -> 2.86x`, geomean `2.42x -> 2.65x`, primary weighted score
  `1.7652 -> 2.0219`, and p99 ratio `3.95x -> 6.15x`. The large-row
  record-size ratio improved in this noisy quick run (`3.95x -> 3.74x`), but
  the whole matrix and write-single category rejected a default-mode switch.
- Do not switch the default WAL path from conservative to auto lane staging as
  a standalone INSERT optimization. Revisit only with a selective policy that
  improves the same-window insert matrix and preserves the lane-staging
  correctness/shadow-compare contract.

## 2026-05-05 - Strict last-60-day CASS resweep

Scope: user-requested resweep for abandoned or losing optimization ideas,
restricted to last-60-day CASS history tied to `/data/projects/frankensqlite`.
The direct CASS workspace filter was stale/sparse and returned zero hits for
the first negative terms, so the pass used the stricter explicit-path session
set: sessions found by
`cass search '/data/projects/frankensqlite' --days 60 --robot-format sessions`.

- Direct workspace spot checks:
  `cass search 'rejected' --workspace /data/projects/frankensqlite --days 60`,
  plus `reverted`, `slower`, and `didn't help`, returned no hits.
- Explicit-path seed set: `51` sessions. Negative vocabulary searched inside
  that set included `rejected`, `reverted`, `slower`, `regressed`,
  `didn't help`, `did not help`, `abandoned`, `abandones`, `within noise`,
  `no improvement`, `rollback`, `worse`, `failed to improve`, `not worth`,
  `gave up`, `no measurable`, `keep gate`, `failed the keep`, `rolled back`,
  and `backed out`.
- Focused perf query pass searched combinations such as `perf rejected`,
  `benchmark slower`, `candidate rejected`, `matrix regressed`,
  `weighted score regressed`, `not a keep`, and `do not retry`.
- No new artifact-backed rejected performance shapes were found beyond entries
  already present in this ledger. Useful hits routed back to existing no-retry
  fences: broad VDBE/public `SmallVec` sweeps, stale raw `bench_insert`
  optimization work, page-1 synthetic hint state, WAL publication/checksum
  candidates, direct INSERT row-build candidates, and benchmark-policy rejects.
- Excluded hits were multi-repo commit/sync sessions, issue triage summaries,
  accepted correctness fixes, ephemeral-file decisions, or negative words from
  skill text rather than FrankenSQLite performance candidates.
- CASS index state at the time of this resweep was stale but usable
  (`database.exists=true`, `index.stale=true`, no active rebuild). Refresh the
  index before relying on this note for sessions created after this date.

## 2026-05-05 - SharedTxnPageIo cached page-size borrow removal

- Target: large INSERT write path after
  `tests/artifacts/perf/insert-large-current-cyangorge-20260505T221825Z/`
  showed `SharedTxnPageIo::write_page_data`/`write_page_internal` under the
  retained rightmost-leaf append profile.
- Candidate shape: commit `16b1907d` cached `TransactionKind::page_size()` in a
  shared `Cell<usize>` inside `SharedTxnPageIo`, updated it on `refill`, and
  used it in `PageWriter::write_page` / `write_page_data` to avoid a hot-path
  `RefCell` borrow before page-data normalization.
- Evidence:
  - Baseline/current profile:
    `tests/artifacts/perf/insert-large-current-cyangorge-20260505T221825Z/report.json`
    and `stderr-insert.log`.
  - Candidate:
    `tests/artifacts/perf/shared-txn-page-size-cache-cyangorge-20260505T2230Z/report.json`
    and `stderr.log`.
- Correctness smoke before measurement: `cargo fmt --check` and
  `env CARGO_TARGET_DIR=.rch-target cargo check -p fsqlite-vdbe -p fsqlite-core`
  passed.
- Result: rejected and reverted. The insert-only weighted score regressed
  `1.6759 -> 1.6944`, average ratio regressed `2.3289x -> 2.3467x`, p99
  regressed `3.7410x -> 3.9032x`, and write-single geomean regressed
  `1.4928x -> 1.5151x`. The target large rows did not improve:
  single-transaction `large_10col` profile had commit roundtrip worsen
  `16.27 ms -> 17.06 ms`, and record-size `large_10col` had B-tree insert
  worsen `7.53 ms -> 8.48 ms` with commit roundtrip still about `17.05 ms`.
- Do not retry caching `page_size` on `SharedTxnPageIo` as a standalone
  optimization. The borrow was visible in code but not a matrix-level bottleneck.

## 2026-05-05 - SharedTxnPageIo staged page-data take/restore forwarding

- Target: retained rightmost-leaf append/write-page cost in the INSERT matrix,
  after current profiles showed time in B-tree insert, row-build, commit-frame
  prep, and `TransactionKind::write_page_data`.
- Candidate shape: expose `try_take_staged_page_data` and
  `restore_staged_page_data` through `TransactionKind` and `SharedTxnPageIo` so
  existing B-tree retained-page mutation paths could take an owned staged page
  image, mutate it, and restore it through the transaction instead of cloning
  from the shared page path. Source was reverted after measurement.
- Correctness proof passed in the candidate checkout:
  `env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-vdbe shared_txn_page_io -- --nocapture`
  (`15` tests), and
  `env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree test_table_try_append_cached_rightmost_leaf_hint -- --nocapture`
  (`4` tests). The release-perf benchmark binaries were rebuilt with
  `env CARGO_TARGET_DIR=.rch-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`.
- Evidence artifacts:
  - Baseline:
    `tests/artifacts/perf/insert-profile-current-20260505T224216Z-proudanchor/report.json`.
  - Candidate:
    `tests/artifacts/perf/staged-take-candidate-insert-20260505T225103Z-proudanchor/report.json`
    plus stdout/stderr logs.
- Result: rejected. The same-host insert matrix regressed on the target rows:
  single-transaction `large_10col` 10K FSQLite median worsened
  `35.246530 ms -> 37.698371 ms`, record-size `large_10col` 10K worsened
  `33.783470 ms -> 37.346692 ms`, `small_3col` 100 single transaction
  worsened `0.212037 ms -> 0.295543 ms`, and `tiny_1col` 100 single
  transaction worsened `0.260568 ms -> 0.280265 ms`.
- Do not retry staged page-data take/restore forwarding through
  `TransactionKind` / `SharedTxnPageIo` as a standalone INSERT optimization.
  Revisit only with a design that mutates in place without remove/restore and
  write-page round trips, then improves absolute FrankenSQLite medians on the
  current prepared INSERT matrix.

## 2026-05-05 - WAL prepared checksum-transform copy removal

- Target: prepared WAL commit/frame-finalization overhead after INSERT profiles
  showed multi-ms `commit_prepare_us`, `commit_batch_build_us`, and
  `commit_flush_frame_prep_us` on 10K large-record rows.
- Candidate shape: in
  `crates/fsqlite-core/src/wal_adapter.rs::finalize_prepared_batch_against_current_state`,
  pass the prepared checksum transforms directly to
  `WalFile::finalize_prepared_frame_bytes` instead of copying them into a fresh
  `Vec<WalChecksumTransform>`. Source was reverted after measurement.
- Correctness proof passed:
  `rch exec -- env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-core --lib append -- --nocapture`
  (`17` tests).
- Evidence artifacts:
  `tests/artifacts/perf/wal-transform-slice-cyangorge-20260505T2350Z/report.json`,
  `report-repeat.json`, and `summary.md`; baseline
  `tests/artifacts/perf/insert-current-head-profile-cyangorge-20260505T2340Z/report.json`.
- Result: rejected. The first candidate run improved weighted INSERT score
  `1.7491 -> 1.6877` but worsened write-bulk geomean `2.4461x -> 2.4612x`
  and p99 `4.0792x -> 4.3231x`; the repeat regressed weighted score
  `1.7491 -> 1.7528` and write-bulk geomean `2.4461x -> 2.5072x`.
- Do not retry removing the prepared checksum-transform copy as a standalone
  WAL optimization unless a fresh profile proves the copy dominates and an
  interleaved A/B improves both weighted INSERT score and write-bulk geomean.

## 2026-05-06 - External rightmost-hint page-image single authority

- Target: prepared monotonic INSERT into large `large_10col` rows, where current
  profiles show thousands of retained right-edge quick-balance events and
  multi-ms B-tree insert / quick-balance / commit-frame costs.
- Candidate shape: in
  `crates/fsqlite-btree/src/cursor.rs::try_quick_balance_on_external_rightmost_leaf_hint`,
  move the newly split rightmost leaf page image into the caller-owned
  `TableAppendHint` and clear the cursor-local rightmost cache, avoiding the
  extra `PageData` clone used to populate both structures. A focused B-tree
  test assertion covered that the caller-owned hint remains authoritative after
  the parent-hint quick-balance path. Source was reverted after measurement.
- Correctness proof passed on the candidate:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-btree-local cargo test -p fsqlite-btree test_table_try_append_cached_rightmost_leaf_hint -- --nocapture`
  (`4` tests), `cargo fmt --check`,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-btree-local cargo check -p fsqlite-btree --lib`,
  and `ubs crates/fsqlite-btree/src/cursor.rs`.
- Evidence artifacts:
  - Baseline:
    `tests/artifacts/perf/right-edge-cache-baseline-crimsongorge-20260506T0450Z/report.json`
    plus stdout/stderr logs.
  - Candidate:
    `tests/artifacts/perf/right-edge-cache-candidate-crimsongorge-20260506T0458Z/report.json`
    plus stdout/stderr logs.
- Result: rejected and reverted. The same-window INSERT matrix improved
  average ratio `2.0991x -> 2.0569x`, geomean `1.9734x -> 1.9378x`, p90
  `3.0645x -> 2.9145x`, and p99 `4.2751x -> 4.1235x`, but the primary
  insert weighted score regressed `1.6318 -> 1.6344`. The critical rows did
  not move enough: single-transaction `large_10col` 10K FSQLite median worsened
  `39.899 ms -> 40.206 ms`, record-size `large_10col` 10K worsened
  `39.701 ms -> 40.862 ms`, and `small_3col` 10K worsened
  `5.176 ms -> 5.875 ms`.
- Do not retry external-hint clone removal / cursor-local cache clearing as a
  standalone B-tree optimization. Revisit only as part of a true monotonic bulk
  append builder that removes per-row quick-balance and page-image churn
  together, then proves absolute `large_10col` 10K medians and the weighted
  INSERT score in the same A/B window.

## 2026-05-06 - Direct UPDATE/DELETE retained table-seek hint

- Scope: direct-simple UPDATE/DELETE rowid probes in `UPDATE/DELETEThroughput`,
  after profiles showed repeated B-tree seek/page work and legacy SQLite keeps
  VDBE cursor position across repeated rowid probes.
- Touched during rejected candidate in a clean worktree only:
  `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`; source was not applied to the shared
  checkout.
- Candidate shape: expose a tiny opaque `TableSeekHint` from `BtCursor` and
  store it on `PreparedStatement` with the current concurrent session id, so
  fresh direct DML cursors can try the previous/successor leaf before falling
  back to root-to-leaf descent. This deliberately did not retain `BtCursor` or
  `SharedTxnPageIo`.
- Correctness/build proof before measurement:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-seek-hint-target cargo check -p fsqlite-core --lib`
  passed, and the candidate `perf-update-delete` plus `comprehensive-bench`
  release-perf binaries built. This was not promoted to a full keep-candidate
  test suite because the section matrix rejected it.
- Evidence artifacts:
  `/data/tmp/frankensqlite-perf-run-20260506T0520Z/seek-hint-delete-fsqlite-isolated.json`,
  `/data/tmp/frankensqlite-perf-run-20260506T0520Z/seek-hint-update-fsqlite-isolated.json`,
  `/data/tmp/frankensqlite-perf-run-20260506T0520Z/seek-hint-both-10000x200-compare.log`,
  `/data/tmp/frankensqlite-perf-run-20260506T0520Z/comprehensive-update-baseline.json`,
  `/data/tmp/frankensqlite-perf-run-20260506T0520Z/comprehensive-update-candidate.json`,
  and `/data/tmp/frankensqlite-perf-run-20260506T0520Z/repeat/rows.tsv`.
- Result: rejected despite isolated wins. Isolated DELETE improved
  `691.8 ms +/- 8.2 ms` to `380.4 ms +/- 6.8 ms` (`1.82x`), isolated UPDATE
  improved `457.5 ms +/- 17.0 ms` to `389.5 ms +/- 15.1 ms` (`1.17x`), and
  isolated compare improved delete ratio `5.61x -> 3.22x`. But the actual
  `comprehensive-bench --quick --filter update` Section 6 gate did not
  reliably move: first-run geomean ratio worsened `2.628 -> 2.659`, repeated
  5-run averages only modestly improved some absolute FSQLite medians and
  regressed `1000 rows / update 100 rows` (`0.888 ms -> 0.939 ms`) while ratios
  worsened on several rows.
- Do not retry retained table-seek hints as a standalone direct UPDATE/DELETE
  optimization. Revisit only if the comprehensive Section 6 row itself is
  changed to preserve cursor/hint work across the real benchmark shape, and
  require a same-window multi-run matrix improvement, not isolated
  `perf-update-delete` wins.

## 2026-05-06 - Prepared direct INSERT leaf-writer serialization fusion

- Target: prepared monotonic INSERT into `large_10col` rows, especially
  `insertthroughput-record-size-comparison-10k-rows-single-txn` where the
  private-memory baseline still showed FSQLite `27.575381 ms` versus C SQLite
  `9.600697 ms` (`2.8722x`), with profiles dominated by row building,
  serialization, B-tree insert, and quick-balance work.
- Candidate shape: carry a `PreparedDirectInsertRecordPlan` through the direct
  INSERT path and expose a retained rightmost-leaf writer primitive on the
  B-tree cursor, so append-fast rows can serialize directly into the leaf page
  when the cached right-edge proof is still valid. Full/split rows fall back to
  the existing serialized-record slice path. Source was reverted after the
  section matrix rejected it.
- Correctness/build proof passed on the candidate:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-writer-plan cargo check -p fsqlite-btree --lib`,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-writer-plan cargo check -p fsqlite-core --lib`,
  and
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-writer-plan cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifacts:
  - Baseline:
    `tests/artifacts/perf/private-memory-journal-candidate-crimsongorge-20260506T0514Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/direct-record-writer-candidate-crimsongorge-20260506T0558Z/report.json`
    and
    `tests/artifacts/perf/direct-record-writer-candidate-repeat-crimsongorge-20260506T0603Z/report.json`.
- Result: rejected. The target record-size `large_10col` 10K row improved in
  candidate runs (`27.575381 ms -> 18.973337 ms`, repeat `19.623204 ms`), but
  the primary INSERT weighted score regressed (`1.4977 -> 1.5576`, repeat
  `1.5110`) and the repeat write-bulk geomean regressed
  (`1.6920 -> 1.7143`). Profiles also showed the direct writer did not remove
  the real row-build cost: row build stayed around `6.1-6.2 ms`, and split /
  fallback rows still paid about `1.7-1.8 ms` of serialization.
- Do not retry direct retained-leaf serialization fusion as a standalone
  optimization. Revisit only inside a true monotonic bulk append/page builder
  that removes per-row quick-balance, page-image churn, and row-template
  construction together, then wins both absolute `large_10col` medians and the
  same-window INSERT weighted score.

## 2026-05-07 - Retained autocommit repeated dirty-table mark fast path

- Target: `INSERTThroughput — Transaction Strategy Comparison (small_3col)`,
  especially autocommit write-single rows where profiles showed repeated direct
  INSERT work under retained autocommit.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, add an early
  return to `Connection::retained_autocommit_mark_dirty` when the table name is
  already lowercase, the retained count/sum cache and indexed-equality cache are
  absent, no preserve-next-write flag is set, and the table is already in the
  retained dirty set. A focused safety test proved the fast path must stay below
  cache invalidation: if the indexed-equality cache is populated, a repeated
  dirty mark still clears it; mixed-case names still canonicalize through the
  existing path.
- Correctness proof passed on the candidate:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dirty-mark-local-target cargo test -p fsqlite-core test_retained_autocommit_dirty_mark_repeated_table_still_clears_overlay_cache -- --nocapture`.
  Source was reverted after the current-HEAD matrix rejected it.
- Evidence artifacts:
  `tests/artifacts/perf/insert-txn-perf-purpleotter-20260507T0816Z/current0f6-baseline-dirtymark-transaction.json`,
  `tests/artifacts/perf/insert-txn-perf-purpleotter-20260507T0816Z/current0f6-candidate-dirtymark-transaction.json`,
  plus stdout/stderr logs in the same directory. The current source baseline and
  candidate worktrees were both built from `0f6a2fd6`, with the candidate
  carrying only this `connection.rs` patch.
- Result: rejected. On the current `0f6a2fd6` transaction section, the primary
  weighted score regressed `1.1329 -> 1.1551`, geomean regressed
  `1.0102 -> 1.0731`, and write-bulk geomean regressed `0.9216 -> 1.0117`.
  Some target autocommit medians improved (`1000` rows
  `1.136870 ms -> 1.100120 ms`, `10000` rows
  `11.397552 ms -> 10.866708 ms`), but batched/single transaction rows
  regressed enough to fail the section keep gate.
- Do not retry repeated dirty-table mark elision as a standalone retained
  autocommit optimization. Revisit only if the dirty/invalidation state is
  redesigned so write-only batches can prove cache absence without extra borrow
  checks, and require same-window improvement in transaction primary score,
  write-bulk geomean, and write-single geomean before a full quick matrix.

## 2026-05-07 - Staged table-leaf delete mutation before clone fallback

- Target: `comprehensive-bench --quick --filter update` UPDATE/DELETE
  throughput matrix, focused on direct DELETE rows.
- Candidate shape: in `crates/fsqlite-btree/src/cursor.rs`, before the
  existing compact table-leaf delete cloned and rewrote `page_data`, check
  whether the leaf page is already staged in the pager and mutate that staged
  page in place, delaying the fallback page clone until after the staged check.
  Source was reverted after measurement.
- Correctness/build evidence passed on the candidate:
  `cargo fmt -p fsqlite-btree`,
  `cargo test -p fsqlite-btree table_delete -- --nocapture`,
  `cargo test -p fsqlite-btree cursor_delete -- --nocapture`,
  `cargo test -p fsqlite-btree insert_delete -- --nocapture`, and a candidate
  e2e binary built from clean detached worktree
  `/data/tmp/frankensqlite-btree-delete-staged-20260507T1214Z` because the
  shared `connection.rs` was dirty during the run.
- Evidence artifacts:
  `tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/report-update-delete-staged-delete-baseline*.json`,
  `tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/report-update-delete-staged-delete-candidate*.json`,
  and matching stdout/stderr files. The rejection is also summarized in
  `tests/artifacts/perf/update-delete-profile-crimsongorge-20260507T111220Z/summary.md`.
- Result: rejected. Average section geomean ratio worsened from
  `1.2061965067269436` baseline to `1.2818079817497139` candidate. FSQLite-only
  median average improved from `1.7101946111111108 ms` to
  `1.5841585555555557 ms`, but the C-relative matrix worsened on five of six
  rows on average; only `10000 rows / delete 500 rows` improved
  (`1.2050318401507203 -> 1.1801234201419353`).
- Do not retry staged-page table-leaf delete mutation as a standalone
  optimization. Revisit only as part of a larger retained-cursor or
  leaf-batched delete kernel that removes per-row root descent and compacts
  each touched leaf once.

## 2026-05-07 - Connection-only retained fixed REAL UPDATE run

- Target: isolated direct UPDATE/DELETE rowid workloads, especially repeated
  monotone `UPDATE ... SET value = ? WHERE id = ?` on a fixed-width REAL
  column where profiles showed remaining per-row cursor/seek work after VDBE
  dispatch had already been bypassed.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, buffer monotone
  explicit-transaction direct UPDATEs of one fixed-width REAL column when the
  exact `MemDatabase` row mirror proves row existence, then flush the buffered
  run with one retained B-tree cursor at read, commit, savepoint, release, DDL,
  and table-program boundaries. The source candidate was reverted after the
  isolated matrix rejected it.
- Correctness proof passed on the candidate:
  `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_fixed_real_update_run_flushes_on_read_and_commit -- --nocapture`,
  plus existing direct UPDATE/DELETE guards:
  `test_direct_simple_update_single_real_column_patches_payload_without_decode`,
  `test_direct_simple_update_delete_fast_path_executes_and_is_correct`, and
  `test_fast_path_update_delete_ddl_invalidation`.
- Evidence artifacts:
  `tests/artifacts/perf/retained-direct-real-update-purpleotter-20260507T123741Z/summary.md`
  and the raw `perf-update-delete` baseline/candidate logs in the same
  directory. Baseline and candidate worktrees were both built from
  `5b36871d`, and the candidate carried only the `connection.rs` patch.
- Result: rejected. Saved isolated timings showed update 100 at
  `656 ns -> 642 ns`, update 1000 at `869 ns -> 838 ns`, but update 10000
  regressed `888 ns -> 910 ns`. Untargeted delete rows also moved the wrong
  way (`1123 ns -> 1247 ns`, `1176 ns -> 1247 ns`,
  `1254 ns -> 1289 ns`), so the candidate failed the real keep gate despite a
  focused correctness win.
- Do not retry connection-only fixed-REAL UPDATE buffering as a standalone
  optimization. Revisit only if profiling first proves the real workload keeps
  the exact row mirror hot for long monotone runs and a same-window isolated
  matrix improves update 1000/10000 without delete regressions.

## 2026-05-07 - Non-empty page-run writer flush replay

- Target: `INSERTThroughput - Transaction Strategy Comparison (small_3col)`,
  especially `10000 rows / batched (1000/txn)` where profiles showed repeated
  right-edge cursor/setup and append work.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, allow direct
  INSERT page-runs to start on a non-empty right edge when the next explicit
  rowid is greater than the table's last rowid, then flush records through the
  existing `table_append_after_last_position_with_writer` payload-writer kernel
  before falling back to byte-slice append. Source was reverted after the
  transaction keep gate rejected it.
- Correctness proof passed on the candidate:
  `cargo fmt -p fsqlite-core` and
  `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-pagebuilder-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core prepared_direct_insert_page_run -- --nocapture`.
- Evidence artifacts:
  `tests/artifacts/perf/right-edge-pagebuilder-purpleotter-20260507T125157Z/summary.md`,
  `baseline-transaction.json`, `candidate-transaction.json`, and matching
  stdout/stderr logs. Baseline worktree was built from `7660b8da`; candidate
  carried only the `connection.rs` patch. Supplemental read-only confirmation:
  `tests/artifacts/perf/payload-writer-pagerun-crimsongorge-20260507T1251Z/summary.md`
  and `candidate-insert.json`.
- Result: rejected. The primary weighted score improved
  `0.9403114839 -> 0.9287374158`, but geomean regressed
  `0.9786733776 -> 1.0159826210` and write-bulk geomean regressed
  `1.0104867009 -> 1.0916437903`. The target row worsened in absolute time and
  ratio: `10000 rows / batched (1000/txn)` moved from `4.548586 ms` /
  `1.3666644733` to `4.686143 ms` / `1.4288776944`. The candidate reduced
  `cursor_setup_ns` and `btree_insert_ns`, but moved work into commit
  (`commit_us=160.0 -> 3186.4`).
- Do not retry non-empty page-run buffering plus writer-flush replay as a
  standalone connection-level optimization. Revisit only with a true page
  builder that lays out the non-empty right-edge run and parent updates in one
  batch, and require an absolute FSQLite median improvement on
  `10000 rows / batched (1000/txn)` before any full-matrix repeat.

## 2026-05-07 - Direct DML fixed-width UPDATE leaf hint

- Target: `comprehensive-bench --quick --filter update` UPDATE/DELETE
  throughput, especially repeated `UPDATE bench SET value = ? WHERE id = ?`
  on the fixed-width REAL direct-simple path.
- Candidate shape: commit `6e13684f` added `PreparedDirectDmlLeafHint`
  (`root_page` + `leaf_page`) on `Connection` and
  `BtCursor::table_move_to_leaf_hint`, so a same-size fixed-width REAL payload
  overwrite could seed a leaf-page hint for the next UPDATE against the same
  table root. The hint was cleared on direct INSERT, direct DELETE, mixed-shape
  UPDATE, and delete+insert fallback paths. Source was reverted after the
  focused matrix rejected it.
- Correctness proof passed on the candidate:
  `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-leafhint-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-btree test_table_move_to_leaf_hint_uses_hinted_leaf_when_bounds_match -- --nocapture`
  and
  `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-leafhint-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core direct_simple_update -- --nocapture`.
- Evidence artifacts:
  `tests/artifacts/perf/direct-dml-leaf-hint-calmdeer-20260507T132114Z/summary.md`,
  `baseline-update.json`, `candidate-update.json`,
  `baseline-update-repeat.json`, `candidate-update-repeat.json`, and matching
  build/stdout/stderr logs. Baseline was built from parent `5af003c1`; candidate
  was built from `6e13684f`.
- Result: rejected. Primary focused run improved only the tiny update row
  (`0.144170 ms -> 0.130484 ms`) while regressing the larger update rows and
  all delete rows: `1000 rows / update 100 rows` moved
  `0.439974 ms -> 0.452818 ms`, `10000 rows / update 1000 rows` moved
  `3.985014 ms -> 4.497334 ms`, and `100 rows / delete 5 rows` moved
  `0.123081 ms -> 0.151113 ms`. The repeat left update rows noisy/mixed and
  still worsened small delete rows (`0.122349 ms -> 0.163867 ms`,
  `0.395791 ms -> 0.420177 ms`).
- Do not retry a connection-level "last leaf page" hint as a standalone direct
  UPDATE optimization. Revisit only as part of a true retained-cursor direct DML
  kernel that keeps the cursor object positioned across a monotone rowid run,
  proves no delete-path overhead in the same benchmark slice, and improves
  update 1000/10000 plus the UPDATE/DELETE section score in same-window runs.

## 2026-05-07 - Param-one concat direct INSERT encoder

- Target: `INSERTThroughput - Single Transaction - medium_6col`,
  especially `1000 rows`, after profiling showed row construction as the
  largest remaining in-row hot slice for the measured gap.
- Candidate shape: in `crates/fsqlite-core/src/connection.rs`, compile
  text-literal/`?1` concat chains such as `'prefix_' || ?1 || '_suffix'` into a
  compact `ParamOneTextConcat(Vec<String>)` prepared-direct expression. The
  encoder reused the already cached text form of integer `?1`, preserved SQLite
  NULL concat semantics, and fell back to the existing `ConcatChain` for every
  other expression shape. Source was reverted after the full quick matrix
  rejected it.
- Correctness proof passed on the candidate:
  `cargo fmt -p fsqlite-core --check` and
  `env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-next-gap-check-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_prepared_insert_ -- --nocapture`.
- Evidence artifacts:
  `tests/artifacts/perf/medium-single-gap-crimsongorge-20260507T1515Z/insert-profile.json`,
  `insert-paramconcat-candidate.json`, `full-paramconcat-candidate.json`, and
  matching stdout/stderr logs.
- Result: rejected. The focused insert matrix moved the target row in the
  right direction (`medium_6col / 1000 rows` FSQLite median
  `0.742511 ms -> 0.684553 ms`, ratio `1.3865 -> 1.2507`) and reduced its
  attributed `row_build_ns` (`248814 -> 201460`). The full quick matrix still
  failed the project keep gate: average ratio worsened `0.496253 -> 0.503004`,
  C-faster rows moved `13 -> 14`, and p99 worsened `1.509348 -> 1.536446`.
- Do not retry a param-one-only concat expression variant as a standalone
  optimization. Revisit only if it is folded into a broader row-template encoder
  that proves full-matrix neutrality or better, not just a single insert-row
  win.

## 2026-05-07 - Direct DELETE no-rebalance leaf primitive

- Target: `UPDATE/DELETEThroughput`, especially direct-simple DELETE rows where
  generic `BtCursor::delete` pays separator/anchor ceremony even when the
  current leaf will remain non-empty and the deleted cell is not the leaf max.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`; source was manually restored after
  the focused DML gate rejected the change.
- Candidate shape: add a narrow table-leaf DELETE primitive that accepts only
  non-max table leaf cells on leaves with more than one cell, calls the existing
  eager-defrag `remove_table_cell_from_leaf_deferred`, and falls back to generic
  `delete()` for all structural/separator/rebalance cases.
- Evidence artifacts:
  `tests/artifacts/perf/direct-delete-leaf-no-rebalance-tanbear-20260507T2055Z/summary.md`,
  `baseline-update.json`, `candidate-update.json`, and `stdout/`.
- Result: rejected and reverted. Focused DML average/geomean worsened
  `1.1771902588843643 / 1.1533073165550498` to
  `1.2530378457971052 / 1.225724573854313`. The 100-row DELETE row improved
  slightly (`0.118893 ms -> 0.116718 ms`) and 10K rows improved, but
  `1000 rows / delete 50 rows` regressed sharply
  `0.353662 ms -> 0.577712 ms`, so the section keep gate failed.
- Do not retry non-max/no-rebalance table-leaf DELETE bypass as a standalone
  direct DELETE optimization. Reconsider only as part of a real same-leaf batch
  mutation primitive that writes each leaf once and proves an UPDATE/DELETE
  section geomean win.

## 2026-05-07 - Same-leaf fixed-width REAL UPDATE batch run

- Target: `UPDATE/DELETEThroughput`, especially monotone explicit-transaction
  fixed-width REAL direct UPDATE rows that seemed eligible for one-write-per-leaf
  batching.
- Touched during rejected shared-worktree candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-btree/src/cursor.rs`; those source files were exclusively
  reserved by TanBear during CrimsonGorge's read-only benchmark review.
- Candidate shape: buffer monotone fixed-width REAL direct UPDATE records in an
  explicit transaction, keep a pending run on `Connection`, and flush through a
  new `BtCursor::table_overwrite_sorted_payloads_same_size_no_overflow`
  primitive that patches same-size no-overflow payloads and writes each dirty
  leaf once. The candidate also carried MemDatabase mirror handling and focused
  btree/core regression tests.
- Correctness proof passed read-only on the current dirty candidate:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-dml-current-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_fixed_real_update_run_flushes_on_read_and_commit -- --nocapture`,
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-dml-current-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-btree test_table_overwrite_sorted_payloads_same_size_no_overflow -- --nocapture`,
  `git diff --check -- crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs`,
  and `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-dml-current-target CARGO_BUILD_JOBS=10 cargo fmt --check`.
- Evidence artifacts:
  `tests/artifacts/perf/dml-batch-current-review-crimsongorge-20260507T2200Z/summary.md`
  and `stdout/perf-update-delete-*.out`.
- Result: rejected by the isolated mutation keep gate. Against the prior
  isolated baseline in
  `tests/artifacts/perf/update-delete-isolated-current-tanbear-20260507T1544Z/summary.md`,
  UPDATE regressed at every measured size: `100` rows moved
  `788 ns/row -> 1079 ns/row`, `1000` rows moved
  `913 ns/row -> 1409 ns/row` with a repeat at `1381 ns/row`, and `10000`
  rows moved `916 ns/row -> 1636 ns/row`. DELETE was noise-flat to worse:
  `1233 -> 1203`, `1209 -> 1200/1235`, and `1328 -> 1364 ns/row`.
- Do not retry a connection-level pending fixed-width REAL UPDATE run plus
  same-size leaf overwrite as a standalone DML optimization. Reconsider only if
  the design also removes the per-row admission/payload projection/mirror costs
  or is replaced with a true leaf-run operator that proves an isolated
  UPDATE/DELETE win before any broader matrix run.

## 2026-05-08 - Global page-buffer recycle capacity 256 -> 2048

- Target: page-buffer allocator churn in INSERT-heavy workloads, especially
  large 10-column inserts whose profile showed `page_pool_misses=2006` with the
  256-entry global recycle cap.
- Touched during landed/shared candidate: `crates/fsqlite-pager/src/page_buf.rs`
  (`GLOBAL_PAGE_BUF_RECYCLE_CAPACITY` raised from `256` to `2048` in
  `41a950b6`).
- Candidate shape: increase the bounded global `PageBuf` recycle list so a
  wider dropped-pool working set can be retained before falling back to the
  allocator. This was intentionally distinct from the previously rejected
  batched-drain locking change.
- Correctness proof passed before benchmark rejection:
  `cargo fmt -p fsqlite-pager --check` and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-pagebuf-cap2048-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-pager page_buf -- --nocapture`.
- Evidence artifacts:
  `tests/artifacts/perf/windyibis-pagebuf-cap2048-20260508T062615Z/insert-profile.json`
  and `tests/artifacts/perf/windyibis-head-0f1b85eb-full-quick-20260508T0700Z/full-quick.json`.
- Result: rejected by the focused INSERT gate. Against the prior keeper
  artifact, focused INSERT worsened on every summary guard:
  weighted score `0.7767315568388111 -> 0.8170165218916904`,
  average `0.7714626475032516 -> 0.9271209597934807`,
  geomean `0.7459511333726486 -> 0.8792631315813308`, p90
  `1.0760814249363868 -> 1.2516524592352403`, and p99
  `1.0984144114228789 -> 2.042096610168269`. The target large 10-column
  record-size row regressed from prior keeper ratio `~1.10x` to `2.04x`
  slower (`9.706411 ms` C SQLite vs `19.821429 ms` FrankenSQLite), and the
  profile still showed `page_pool_misses=2006`, so the larger cap did not
  address the measured miss source.
- Supporting full quick check at `0f1b85eb` was also worse than the prior
  keeper: weighted score `0.3358994390491727 -> 0.3552972206567397`, average
  `0.4420352710879217 -> 0.49071440245809644`, and p99
  `1.2422341250364553 -> 2.2697406591196656`. That run reported the harness
  "binary predates Git HEAD" warning because the commit timestamp postdated the
  release-perf binary mtime, though the changed source mtimes predated the
  binary.
- Do not retry a larger single global page-buffer recycle cap as a standalone
  optimization. Revisit only with an isolated allocator proof that actually
  reduces `page_pool_misses` or allocator samples on the large-row workload, and
  keep it only if the focused INSERT and full quick matrix both improve.

## 2026-05-08 - Pending direct DELETE leaf-run buffer via repeated seeks

- Target: the remaining `UPDATE/DELETEThroughput` 100-row DELETE tail, after
  the clean frontier showed the gap is a real direct-DML mutation-kernel cost
  rather than population/setup time.
- Touched during rejected shared-worktree candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-btree/src/cursor.rs`; both files were exclusively reserved by
  WindyIbis during SilverAnchor's read-only smoke review.
- Candidate shape observed in the dirty worktree: buffer proven monotone
  prepared direct DELETE rowids in `PendingDirectDeleteLeafRun`, retain the
  current leaf page and maximum rowid, and flush through
  `BtCursor::table_delete_current_leaf_rowids_no_rebalance` so the leaf is
  compacted and written once at the next observation boundary. Each row still
  performs the ordinary root-to-leaf seek before being admitted to the pending
  run.
- Correctness proof available before rejection: read-only compile passed with
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-leafrun-check CARGO_BUILD_JOBS=12 cargo check -p fsqlite-btree -p fsqlite-core --lib`.
- Evidence artifact:
  `tests/artifacts/perf/silveranchor-wal-pipeline-review-20260508T1115Z/summary.md`.
- Result: rejected by the isolated mutation smoke. The dirty candidate measured
  `perf-update-delete 100 20000 delete fsqlite isolated` at `3203ns/delete`,
  while the current clean baseline from
  `tests/artifacts/perf/boldlion-setup-mutation-review-20260508T1040Z/summary.md`
  was `1754ns/delete` for the same family. This is about `1.83x` slower before
  any focused DML or full-quick gate.
- Do not land or retry this specific pending direct DELETE leaf-run shape as a
  standalone optimization. Reconsider DELETE leaf-run batching only if it avoids
  the per-row root-to-leaf admission cost or otherwise proves an isolated
  DELETE-kernel win before running the focused `UPDATE/DELETEThroughput` matrix.

## 2026-05-08 - Fixed-width REAL UPDATE payload-range page patch

- Target: the remaining `UPDATE/DELETEThroughput` 100-row UPDATE tail, after
  profiling showed `BtCursor::load_page`, staged-page writes, and payload copy
  work inside the direct fixed-width REAL update lane.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-btree/src/cursor.rs`. Agent Mail reservation attempts for
  this narrow surface and the artifact directory timed out before the edit, so
  the candidate was kept short and then manually unwound after rejection.
- Candidate shape: add
  `BtCursor::table_overwrite_current_payload_range_same_size_no_overflow` and
  route `Connection::try_execute_prepared_direct_simple_update_fixed_width_real`
  through a payload-slice patch so the caller does not rebuild and rewrite the
  unchanged prefix/suffix of a fixed-width record.
- Correctness proof passed before benchmark rejection:
  `cargo test -p fsqlite-btree test_table_overwrite_current_payload_range_same_size_no_overflow_patches_slice -- --nocapture`
  and
  `cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture`
  with
  `CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-insert-profile-target`.
- Evidence artifacts:
  `tests/artifacts/perf/silveranchor-dml-profile-20260508T1225Z/dml-profile-candidate.json`,
  `tests/artifacts/perf/silveranchor-dml-profile-20260508T1225Z/dml-profile-clean-head-repeat.json`,
  `tests/artifacts/perf/silveranchor-dml-profile-20260508T1225Z/perf-update-delete-100x1000-candidate.stderr`,
  and
  `tests/artifacts/perf/silveranchor-dml-profile-20260508T1225Z/summary.md`.
- Result: rejected by the focused update/delete matrix against a clean
  same-time `65d54751` baseline. Summary moved the wrong way:
  average ratio `1.0023996891494373 -> 1.121703499721951`, geomean
  `0.9891774494336053 -> 1.0981172488823585`, median
  `0.9521782416313966 -> 1.2005915733983752`, and p90
  `1.3360203368047023 -> 1.4432270168855534`. The 1000-row UPDATE and DELETE
  rows regressed from `0.388989 ms -> 0.488295 ms` and
  `0.347651 ms -> 0.409748 ms`, respectively, despite a small 100-row UPDATE
  absolute improvement.
- Do not retry a B-tree payload-range patch as a standalone fixed-width UPDATE
  optimization. Reconsider only if it also removes the per-row admission/seek
  cost or proves a stable win on the focused update/delete matrix against a
  same-time clean baseline.

## 2026-05-08 - Fixed-width REAL UPDATE leaf-local field patch

- Target: the remaining `UPDATE/DELETEThroughput` UPDATE rows after the prior
  payload-range page patch rejection showed full-payload copying still visible
  in the direct fixed-width REAL update counter path.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-btree/src/cursor.rs`. The source patch was manually unwound
  after the final same-window benchmark gate moved the wrong way.
- Candidate shape: add a btree helper that parses the in-page table record
  header for the currently positioned local, non-overflow row and patches only
  the serial-type-7 REAL field bytes in the staged leaf page. The direct UPDATE
  fast path tried this helper before falling back to the existing full-payload
  overwrite.
- Correctness proof passed before benchmark rejection:
  `cargo fmt --check -p fsqlite-btree -p fsqlite-core`,
  `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p fsqlite-btree test_table_overwrite_current_real_column_same_size_no_overflow_patches_in_place -- --nocapture`,
  and
  `cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture --test-threads=1`.
- Evidence artifact:
  `tests/artifacts/perf/rusticgrove-realpatch-20260508T1246Z/summary.md`.
- Result: rejected by the final isolated same-window update-filter matrix. The
  candidate still removed `btree_payload_copy_calls` from the direct UPDATE rows,
  but FSQLite UPDATE medians regressed against the clean baseline repeat:
  `0.116348 ms -> 0.117840 ms` for 100 rows,
  `0.382126 ms -> 0.401081 ms` for 1000 rows, and
  `3.419356 ms -> 3.583383 ms` for 10000 rows. The first final candidate run
  also had high CV on the 100-row and 1000-row UPDATE rows, so the already-built
  final candidate binary was repeated before rejection.
- Do not retry a leaf-local fixed-width REAL field patch as a standalone
  optimization. Reconsider only if it also removes per-row admission/seek or
  commit-side cost in the same change and wins the focused update/delete matrix
  against a same-time clean baseline.

## 2026-05-08 - Engine-level exact benchmark PRAGMA execute fast path

- Target: fixed setup cost visible in the remaining 100-row INSERT and DML
  tails, specifically the repeated `apply_pragmas_fsqlite` calls before each
  benchmark connection setup.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs`. The source patch and focused tests
  were manually unwound after rejection; only this ledger entry and artifacts
  remain.
- Candidate shape: add a `Connection::execute` pre-parse fast path for the
  exact benchmark setup PRAGMAs:
  `PRAGMA page_size = 4096;`, `PRAGMA journal_mode = WAL;`,
  `PRAGMA synchronous = NORMAL;`, `PRAGMA cache_size = -64000;`, and
  `PRAGMA fsqlite_capture_time_travel_snapshots=false;`. The path was guarded
  to fall back when trace hooks, tracing, retained autocommit state,
  cached-write state, pending direct page-runs, or dirty MemDB refresh state
  could make full statement-boundary dispatch observable. File-backed
  `journal_mode = WAL` also fell back to normal dispatch.
- Correctness proof passed before benchmark rejection:
  `cargo test -p fsqlite-core exact_benchmark_pragma -- --nocapture` through
  RCH passed the memory fast-path test before artifact retrieval was
  interrupted; the file-backed journal fallback test then passed locally. After
  replacing the initial `sql.trim()` guard with exact string matching,
  `cargo test -p fsqlite-core exact_benchmark -- --nocapture` passed locally
  with both focused tests.
- Evidence artifacts:
  `tests/artifacts/perf/swiftgate-pragma-fastpath-20260508T1530Z/candidate-insert.json`,
  `tests/artifacts/perf/swiftgate-pragma-fastpath-20260508T1530Z/candidate-insert-repeat.json`,
  `tests/artifacts/perf/swiftgate-pragma-fastpath-20260508T1530Z/candidate-exact-insert.json`,
  and
  `tests/artifacts/perf/swiftgate-pragma-fastpath-20260508T1530Z/summary.md`.
- Result: rejected by the focused insert matrix. Against the frontier
  `rusticgrove-full-quick-current-20260508T1510Z/insert-profile.json`, the
  exact-match candidate had fewer faster rows and worse distribution metrics:
  faster/comparable/C-faster `17/2/6 -> 14/3/8`, average ratio
  `0.803142 -> 0.911731`, geomean `0.780274 -> 0.876027`, median
  `0.725773 -> 0.879209`, p90 `1.074184 -> 1.141296`, and p99
  `1.132336 -> 1.809162`. The earlier `trim()` guard was also rejected:
  its first run worsened average/geomean/median/p90/p99, and its repeat still
  worsened average/geomean/median/p90/p99 despite a better weighted subscore.
- Do not retry an engine-level exact benchmark PRAGMA fast path as a standalone
  optimization. Reconsider only if benchmark setup is removed without adding an
  `execute` guard to every statement, or if a same-window full quick matrix
  proves that the setup win outweighs the dispatch guard cost.
