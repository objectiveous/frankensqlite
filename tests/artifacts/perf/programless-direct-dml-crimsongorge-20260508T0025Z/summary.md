# Programless direct UPDATE/DELETE prepare

- Agent: CrimsonGorge
- Date: 2026-05-08
- Source candidate: `crates/fsqlite-core/src/connection.rs`
- Baseline binary: `/data/tmp/frankensqlite-dirty-cursor-target/release-perf/comprehensive-bench`
- Candidate binary: `/data/tmp/frankensqlite-programless-dml-bench-target/release-perf/comprehensive-bench`

## Candidate

Direct-simple prepared UPDATE/DELETE statements no longer compile a reusable
table VDBE program during `prepare()`. They keep the parsed deferred statement,
fast-path metadata, and a placeholder program. Normal execution still uses the
direct UPDATE/DELETE lane when the fused-entry controls and tracing state allow
it; forced fallback and tracing/reuse controls execute the deferred statement
path instead of trying to dispatch a placeholder table program.

## Focused UPDATE/DELETE gate

First pair:

- Baseline: avg `1.182202935376568`, geomean `1.166175523739594`, p90 `1.490666935727979`
- Candidate: avg `1.1429769241393246`, geomean `1.1323352486500657`, p90 `1.3586460032626428`

Repeat pair:

- Baseline: avg `1.2083452027698798`, geomean `1.1820782389717988`, p90 `1.7389397167564857`
- Candidate: avg `1.1381892772956286`, geomean `1.1081706142015102`, p90 `1.6087240159221583`

## Full quick matrix

- Baseline: weighted score `0.3465120555842359`, avg `0.46446389476746164`, geomean `0.2680617353809438`, median `0.3050885856991741`, p90 `1.033116157502983`, p99 `2.0846727061365304`
- Candidate: weighted score `0.3362125739594861`, avg `0.4394022088653307`, geomean `0.2602985958918408`, median `0.2888241368777787`, p90 `0.9748330243205917`, p99 `1.3722771794746844`

UPDATE/DELETE full-matrix rows:

| Scenario | Baseline ratio | Candidate ratio |
| --- | ---: | ---: |
| 100 rows / update 10 rows | `1.4120964473861948` | `1.3659488717119994` |
| 100 rows / delete 5 rows | `1.3787398803238295` | `1.3722771794746844` |
| 1000 rows / update 100 rows | `0.9264869466648793` | `0.7201388272864366` |
| 1000 rows / delete 50 rows | `0.9051299137248615` | `0.8541918821181225` |
| 10000 rows / update 1000 rows | `0.8495533613021977` | `0.7929177744697171` |
| 10000 rows / delete 500 rows | `0.7400491717498978` | `0.7476567579621974` |

## Verification

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_update_prepare_skips_compiled_program`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_delete_prepare_skips_compiled_program`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target-2 CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_prepare_update_with_params`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target-3 CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_prepare_delete_with_params`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_prepared_update_delete_precompute_statement_savepoint_skip_hint`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_prepared_update_write_after_write_defers_active_txn_memdb_reload_until_read_boundary`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_programless_prepared_update_delete_forced_fallback_use_deferred_path`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo check -p fsqlite-core --all-targets`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target CARGO_BUILD_JOBS=10 cargo clippy -p fsqlite-core --all-targets -- -D warnings`
- `cargo fmt -p fsqlite-core --check`

Note: an overly broad `cargo test -p fsqlite-core prepare -- --nocapture`
run was stopped after it pulled in unrelated logging-heavy tests and known
unrelated failures. The targeted prepared-DML tests above were then run cleanly.

UBS was also run on `crates/fsqlite-core/src/connection.rs` and this summary
file. It completed after 607s with exit code 1 because the huge pre-existing
`connection.rs` inventory still contains broad panic/unwrap/SQL-construction
findings; no new finding was identified in the changed programless-DML lines.
