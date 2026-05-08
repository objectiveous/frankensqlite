# Fixed-width REAL update leaf patch

Date: 2026-05-08
Agent: RusticGrove
Baseline: `0382ee26` (`fix(pager): unpublish staged page one before mutation`)

## Change Under Test

Patch fixed-width REAL direct UPDATE so the fast path patches the leaf-table
record field in place after parsing the in-page record header. The existing
full-payload overwrite remains the fallback for row shapes that are not local,
same-size, non-overflow, exact-column-count REAL fields.

## Artifacts

- `baseline-update-profile.json` / `.stderr`: baseline update-filter quick run.
- `update-profile.json` / `.stderr`: candidate update-filter quick run.
- `baseline-repeat-update-profile.json` / `.stderr`: baseline repeat run after candidate.
- `candidate-repeat-update-profile.json` / `.stderr`: candidate repeat run before baseline repeat.
- `candidate-full-quick-profile.json` / `.stderr`: candidate full quick matrix.
- `candidate-final-update-profile.json` / `.stderr`: isolated final candidate
  after checked-range hardening.
- `candidate-final-repeat-update-profile.json` / `.stderr`: repeat of the
  isolated final candidate binary.
- `baseline-final-repeat-update-profile.json` / `.stderr`: same-window baseline
  repeat for the final verdict.

## Mechanism Proof

The update profile counter moved from one full local-payload copy per updated
row to zero copies on the patched path:

| Run | 100-row UPDATE | 1000-row UPDATE | 10000-row UPDATE |
| --- | ---: | ---: | ---: |
| baseline repeat | 10 calls / 189 bytes | 100 calls / 1989 bytes | 1000 calls / 20889 bytes |
| candidate repeat | 0 calls / 0 bytes | 0 calls / 0 bytes | 0 calls / 0 bytes |

## Update A/B Results

FSQLite median times, lower is better:

| Window | Scenario | Baseline | Candidate | Delta |
| --- | --- | ---: | ---: | ---: |
| A/B 1 | 100 rows / update 10 rows | 0.119934 ms | 0.117089 ms | -2.37% |
| A/B 1 | 1000 rows / update 100 rows | 0.392375 ms | 0.384580 ms | -1.99% |
| A/B 1 | 10000 rows / update 1000 rows | 3.582330 ms | 3.468016 ms | -3.19% |
| A/B 2 | 100 rows / update 10 rows | 0.118943 ms | 0.120336 ms | +1.17% |
| A/B 2 | 1000 rows / update 100 rows | 0.390642 ms | 0.385522 ms | -1.31% |
| A/B 2 | 10000 rows / update 1000 rows | 3.466613 ms | 3.383768 ms | -2.39% |

The small 100-row UPDATE was mixed inside short-run noise; the 1000-row and
10000-row rows initially improved in both same-window comparisons. The
update-filter summary includes DELETE rows, which this patch does not touch.

## Final Isolated Verdict Run

After tightening the candidate to use checked page ranges, the patch was copied
into an isolated worktree at `0382ee26` so the later unrelated e2e filter change
in the main worktree could not affect the benchmark binary. The first final run
had high CV on the 100-row and 1000-row UPDATE rows, so the already-built
candidate binary was repeated and compared with an already-built clean baseline
binary in the same window.

FSQLite median times, lower is better:

| Scenario | Baseline final repeat | Candidate final repeat | Delta |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.116348 ms | 0.117840 ms | +1.28% |
| 1000 rows / update 100 rows | 0.382126 ms | 0.401081 ms | +4.96% |
| 10000 rows / update 1000 rows | 3.419356 ms | 3.583383 ms | +4.80% |

The mechanism proof still held (`btree_payload_copy_calls=0` on candidate), but
the matrix result moved the wrong way. The source patch was manually unwound.

## Candidate Full Quick Matrix

`candidate-full-quick-profile.json`:

- avg ratio: `0.45732272279520764`
- geomean ratio: `0.2681031986820238`
- p90 ratio: `1.0167682562156446`
- p99 ratio: `1.4189438321930437`

Known slower rows remaining in the full quick matrix are the small INSERT tail,
2/4-writer concurrent rows, and 100-row UPDATE/DELETE. The candidate update
profile retained `btree_payload_copy_calls=0` for all fixed-width REAL update
rows in the full matrix.

## Verification

- `cargo fmt --check`: pass before the unrelated e2e dirty change appeared.
- `cargo fmt --check -p fsqlite-btree -p fsqlite-core`: pass on the final
  candidate source before unwind.
- `cargo check --workspace --all-targets`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test -p fsqlite-btree test_table_overwrite_current_real_column_same_size_no_overflow_patches_in_place -- --nocapture`: pass
- `cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture --test-threads=1`: pass
- `cargo test --workspace`: fails on pre-existing `tests::fk_insert_missing_parent_fails`
  in `fsqlite`; the same test also fails at clean baseline `0382ee26`.
- `ubs crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs
  tests/artifacts/perf/rusticgrove-realpatch-20260508T1246Z/summary.md`: exit 1
  from broad pre-existing whole-file findings in the two large Rust files; no
  source patch was kept.

## Verdict

Reject. The patch removed the direct REAL update payload copy on the real
transaction path, but the final isolated same-window matrix regressed the UPDATE
medians. Do not retry this leaf-local REAL field patch as a standalone
optimization unless a new profile shows a way to remove admission/seek or
commit-side cost in the same change.
