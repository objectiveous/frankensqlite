## bd-3wop3.1.2 progress

Current status:
- Read `/data/projects/frankensqlite/AGENTS.md` and `br show bd-3wop3.1.2`.
- Traced the existing lane-local staging implementation in `crates/fsqlite-pager/src/pager.rs`, `crates/fsqlite-wal/src/group_commit.rs`, `crates/fsqlite-wal/src/per_core_buffer.rs`, and `crates/fsqlite-wal/src/parallel_wal.rs`.
- Confirmed the production path already stages prepared WAL batches per lane, logs `fsqlite::wal::lane_staging` events, and routes `auto`, `conservative`, and `shadow_compare` through the pager commit path.

This focused commit adds:
- bead-scoped e2e coverage for auto/conservative/shadow-compare lane staging plus forced `lane_overflow` fallback
- structured-log validation for `wal_lane_id`, backlog, staged frame count, control mode, shadow verdict, compatibility selector, fallback reason, and elapsed time
- the named verification entrypoint `scripts/verify_d1_parallel_wal_staging.sh` with artifact-bundle output

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Verification:
- `cargo test -p fsqlite-pager bd_3wop3_1_2 -- --nocapture` passed locally.
- `cargo test -p fsqlite-e2e --test bd_3wop3_1_2_parallel_wal_staging -- --nocapture --test-threads=1` passed and validated auto/conservative/shadow-compare plus forced `lane_overflow`.
- `cargo check --workspace --all-targets` passed.
- `scripts/verify_d1_parallel_wal_staging.sh` passed and wrote artifacts under `artifacts/bd-3wop3.1.2/bd-3wop3.1.2-20260410T205914Z/`.
- `rustfmt --check crates/fsqlite-e2e/tests/bd_3wop3_1_2_parallel_wal_staging.rs` passed.
- `bash -n scripts/verify_d1_parallel_wal_staging.sh` passed.

Known pre-existing blockers outside this focused change:
- `cargo fmt --check` fails on unrelated untracked file `crates/fsqlite-e2e/tests/bd_abgqx_track_s_register_values.rs`.
- `cargo clippy --workspace --all-targets -- -D warnings` fails on existing `clippy::useless_conversion` in `crates/fsqlite-types/src/record.rs:1430`.
