# Perf Frontier Refresh

Date: 2026-05-08
Agent: WindyIbis
Source status: artifact-only pass. No source edited by WindyIbis.

## Scope

This refresh was run after the latest published artifact:

- `92869f2a docs(perf): publish direct dml source blocker`

During the pass, the peer-owned page-buffer/timing candidate landed as:

- `7563874a perf(pager): restore measured page buffer recycle cap`

Then a peer artifact-only negative-ledger commit landed:

- `d3626ad1 docs(perf): record rejected schema lookup candidate`

A separate peer-owned dirty edit in the following file was present when the
full quick benchmark was run:

- `crates/fsqlite-core/src/connection.rs`

I did not edit or stage that file, and it was gone again before this artifact
commit. The `head756-full-quick.json` benchmark binary was built from
`7563874a`; the JSON reports `git_dirty: true` because the peer-owned
`connection.rs` edit was present at benchmark runtime.

The focused `head756-dml-profile.json` run reports `git_commit_sha=d3626ad1`
and `benchmark_binary_older_than_git_head=true`. That warning is from the
artifact-only peer commit above; no source files changed between `7563874a` and
`d3626ad1`.

## Required Context Read

`README.md` and `AGENTS.md` were re-read in full for this pass.

Important current-state points from `README.md`:

- The stable user-facing runtime is the compatibility/pager-backed path over
  standard SQLite files.
- Native/ECS storage remains partial implementation/design work, not the hot
  path that moves today's benchmark matrix.
- The current public entry point is `fsqlite::Connection`, with most
  table-backed work compiled through `fsqlite-vdbe::codegen` and direct DML
  fast lanes in `fsqlite-core`.

Important constraints from `AGENTS.md`:

- Concurrent-writer mode must remain on by default.
- Performance work must honor the negative-results ledger.
- Rejected candidates need durable ledger entries, but this pass did not create
  a new source candidate.
- Use `bv --robot-*`, `br --json`, and CASS as leads, not as substitutes for
  benchmark evidence.

## Beads And Triage

`br ready --json` and `bv --robot-triage` show a broad queue. The top graph
recommendations are mostly epics (`bd-bje80`, `bd-e77h7`, `bd-hnzcr`,
`bd-y5w0m`) rather than narrow measured performance slices.

DB300-relevant recommendations are broad, blocked, or recently overlapped the
page-buffer/WAL work that landed during this pass:

- `bd-db300.3.8`: persistent 8t/16t benchmark gap, blocked by `bd-db300.3`.
- `bd-db300.9.1`: retry taxonomy instrumentation, blocked by `bd-db300.9`.
- `bd-db300.5.2` and nearby split-phase/publish work likely overlaps WAL and
  executor surfaces touched by the landed page-buffer/timing commit.
- `bd-db300.2.4` bounded handoff/sleep-yield work also points toward WAL/group
  commit behavior that the landed page-buffer/timing commit changed.

## CASS

`cass status --json` reported a stale lexical index. I attempted a refresh:

```text
timeout 600 cass index --json
```

Result: failed after about 252s with:

```text
index failed: inserting historical salvage batch source rows Some(3324)..Some(3324) | out of memory
```

Targeted stale-index searches were still attempted:

- `/data/projects/frankensqlite` returned many repo-path hits but no narrowed
  recent perf session set.
- `frankensqlite rejected reverted slower regressed keep gate direct insert update delete pagebuf group commit`
  returned no hits.
- `frankensqlite db300 100 rows update delete 2 writers pagebuf timing full quick`
  returned no hits.
- `frankensqlite direct dml source blocker leaf run operator row building`
  returned one false lead in an old visualization/beads Gemini session.
- `windyibis pagebuf full quick insert profile frankensqlite` returned no hits.

Conclusion: current CASS is not reliable enough to pick a new source lever in
this pass. The local artifact ledger is stronger evidence.

## Current Benchmark Frontier

Previous dirty full quick frontier before `7563874a` landed:

- `tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-full-20260508T0710Z/full-quick.json`

Summary:

| Metric | Value |
| --- | ---: |
| Primary weighted score | `0.3348866468` |
| Average ratio | `0.4461065430` |
| Geomean ratio | `0.2575107741` |
| P90 ratio | `0.9757275559` |
| P99 ratio | `1.4291042441` |
| Faster / comparable / slower | `81 / 3 / 9` |

Rows above `0.95x` in that matrix:

| Ratio | Section | Row |
| ---: | --- | --- |
| `1.429104` | UPDATE/DELETEThroughput | `100 rows / update 10 rows` |
| `1.389341` | INSERT single txn tiny_1col | `100 rows` |
| `1.138997` | INSERT single txn medium_6col | `100 rows` |
| `1.137822` | INSERT txn strategy small_3col | `100 rows / batched (100/txn)` |
| `1.126649` | Concurrent Writers | `2 writers x 1000 rows` |
| `1.120139` | INSERT txn strategy small_3col | `100 rows / single txn` |
| `1.113675` | INSERT single txn large_10col | `100 rows` |
| `1.110542` | UPDATE/DELETEThroughput | `100 rows / delete 5 rows` |
| `1.063324` | INSERT single txn large_10col | `10000 rows` |
| `0.975728` | Concurrent Writers | `4 writers x 1000 rows` |
| `0.972428` | INSERT record size large_10col | `10K rows` |
| `0.950427` | UPDATE/DELETEThroughput | `1000 rows / update 100 rows` |

Post-landing full quick baseline:

- `tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-full-quick.json`

Summary:

| Metric | Value |
| --- | ---: |
| Source commit | `7563874a02e99881ae5466f4a6c121f3f17d572d` |
| Git dirty in JSON | `true` |
| Primary weighted score | `0.3470296740` |
| Average ratio | `0.4646237940` |
| Geomean ratio | `0.2645872563` |
| Median ratio | `0.3186896618` |
| P90 ratio | `1.0877373564` |
| P99 ratio | `1.6217478587` |
| Faster / comparable / slower | `79 / 3 / 11` |

Rows above `0.95x` in the post-landing matrix:

| Ratio | Section | Row |
| ---: | --- | --- |
| `1.621748` | INSERT txn strategy small_3col | `100 rows / single txn` |
| `1.350608` | UPDATE/DELETEThroughput | `100 rows / update 10 rows` |
| `1.321280` | UPDATE/DELETEThroughput | `100 rows / delete 5 rows` |
| `1.132039` | Concurrent Writers | `2 writers x 1000 rows` |
| `1.131304` | INSERT txn strategy small_3col | `100 rows / batched (100/txn)` |
| `1.117751` | INSERT single txn small_3col | `100 rows` |
| `1.095374` | INSERT single txn large_10col | `100 rows` |
| `1.093230` | INSERT single txn large_10col | `10000 rows` |
| `1.090259` | INSERT record size large_10col | `10K rows` |
| `1.087737` | INSERT single txn tiny_1col | `100 rows` |
| `1.056150` | UPDATE/DELETEThroughput | `10000 rows / update 1000 rows` |
| `1.031915` | INSERT single txn medium_6col | `100 rows` |
| `0.984937` | Concurrent Writers | `4 writers x 1000 rows` |
| `0.983647` | UPDATE/DELETEThroughput | `1000 rows / update 100 rows` |

The page-buffer cap landing kept the matrix broadly faster than C SQLite, but
the tail moved. The largest post-landing p99 row is now the 100-row
single-transaction INSERT strategy row, followed by 100-row direct DML setup
cost and the low-thread concurrent writer row.

Focused post-landing INSERT profile:

- `tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-insert-profile.json`

Summary:

| Metric | Value |
| --- | ---: |
| Scenarios | `25` |
| Average ratio | `0.8372309293` |
| Geomean ratio | `0.8013196058` |
| P90 ratio | `1.1073353870` |
| P99 ratio | `1.6871174266` |
| Faster / comparable / slower | `17 / 4 / 4` |

Rows above `0.95x` in the focused INSERT profile:

| Ratio | C ms | F ms | F CV% | Section | Row |
| ---: | ---: | ---: | ---: | --- | --- |
| `1.687117` | `0.064040` | `0.108043` | `25.92` | INSERT single txn tiny_1col | `100 rows` |
| `1.124640` | `0.073548` | `0.082715` | `4.04` | INSERT single txn small_3col | `100 rows` |
| `1.107335` | `0.072716` | `0.080521` | `3.07` | INSERT txn strategy small_3col | `100 rows / batched (100/txn)` |
| `1.096454` | `0.073237` | `0.080301` | `8.61` | INSERT txn strategy small_3col | `100 rows / single txn` |
| `1.048524` | `0.102402` | `0.107371` | `6.10` | INSERT single txn medium_6col | `100 rows` |
| `1.041240` | `0.936284` | `0.974896` | `34.76` | INSERT single txn large_10col | `1000 rows` |
| `1.034054` | `9.566804` | `9.892594` | `10.96` | INSERT single txn large_10col | `10000 rows` |
| `1.021080` | `9.839585` | `10.047004` | `9.23` | INSERT record size large_10col | `10K rows` |

The focused INSERT profile does not reproduce the full quick's `1.621748x`
`100 rows / single txn` row; it reran at `1.096454x`. The volatile row moved to
`tiny_1col 100 rows`, where FrankenSQLite CV was `25.92%`. Treat the small-N
INSERT tail as noise/setup-dominated until a same-window repeat proves
otherwise.

Focused post-landing UPDATE/DELETE profile:

- `tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-dml-profile.json`

Summary:

| Metric | Value |
| --- | ---: |
| Scenarios | `6` |
| Average ratio | `1.0893517744` |
| Geomean ratio | `1.0564291964` |
| P90/P99 ratio | `1.5942111048` |
| Faster / comparable / slower | `4 / 0 / 2` |
| Binary older than Git HEAD | `true`, artifact-only `d3626ad1` caveat |

Rows in the focused DML profile:

| Ratio | C ms | F ms | F CV% | Row |
| ---: | ---: | ---: | ---: | --- |
| `1.594211` | `0.090069` | `0.143589` | `7.89` | `100 rows / delete 5 rows` |
| `1.362890` | `0.084097` | `0.114615` | `7.59` | `100 rows / update 10 rows` |
| `0.941040` | `0.392706` | `0.369552` | `22.08` | `1000 rows / delete 50 rows` |
| `0.885801` | `3.555191` | `3.149190` | `1.01` | `10000 rows / delete 500 rows` |
| `0.876685` | `0.443602` | `0.388899` | `5.17` | `1000 rows / update 100 rows` |
| `0.875484` | `3.920185` | `3.432060` | `1.52` | `10000 rows / update 1000 rows` |

The DML stdout profiles showed no fallback path: `fast == mutations`,
`slow == 0`, and `vdbe_opcodes == 0` for all six rows. The 100-row rows are
fixed-cost dominated: update logged `setup_us=53.6`, `mutate_us=11.9`,
`commit_us=6.0`; delete logged `setup_us=53.2`, `mutate_us=14.7`,
`commit_us=9.1`.

## Negative Ledger Fences

The obvious standalone source shapes are already rejected:

- Prepared direct INSERT row-template executor.
- Prepared direct INSERT no-FK guard cache.
- Param-one text/int/float/binary expression specializations.
- Direct INSERT fixed cell array staging.
- Direct INSERT page-run admission and broader page-run/bulk-writer variants.
- Direct-record layout reuse, row-value pooling, owned-text moving.
- SharedTxnPageIo cleanup/context probes.
- Same-size direct UPDATE/DELETE staged-page overwrite.
- Retained direct UPDATE/DELETE cursor shell and table-seek hints.
- Direct DELETE no-rebalance leaf primitive.
- Fixed-width REAL direct UPDATE payload patch.
- Direct UPDATE/DELETE scratch reset removal.
- Lazy VDBE fallback/programless variants as standalone retries.
- MVCC one-pass page-state scan and small write-set `SmallVec` probes for the
  low-thread concurrent row.

## Alien-Graveyard Lens

The applicable concepts are useful only if scoped to today's hot path:

- Vectorized execution / AMAC: fits a future batch DML operator, but the current
  public prepared-statement API executes one row at a time.
- B-epsilon trees / message buffers: relevant to write amplification, but not a
  narrow patch for the standard SQLite B-tree compatibility path.
- Parallel WAL and flat combining: relevant to concurrent writers and group
  commit, but the current dirty peer edits already touch WAL/executor timing.
- LeanStore/pointer swizzling/S3-FIFO: relevant to buffer-pool policy, and the
  landed page-buffer cap was the current measured keeper in that family.

The only near-term alien-shaped source idea that is different from the rejected
microprobes is a true batch operator: accept a sorted rowid/value run, walk each
touched leaf once, mutate all same-leaf rows while staged, and publish one page
write per dirty leaf where possible. That is not the same as another retained
cursor shell.

## Decision

The page-buffer/timing integration is now landed, and the focused profiles
change the next-source order. The INSERT p99 row is too volatile to optimize
against directly. The steadier regression is the 100-row UPDATE/DELETE setup
tail.

When the tree is source-clean, next attempts should be tried in this order:

1. Prototype only a true same-leaf batch
   UPDATE/DELETE operator with an isolated proof first. Do not retry cursor
   retention, fixed-width REAL patches, QF gates, or no-rebalance DELETE.
2. If the setup phase remains dominant, investigate a broader prepared
   statement setup redesign. Do not retry the schema side-index lookup from
   `d3626ad1` as a standalone change.
3. If INSERT becomes the larger weighted tail in a same-window repeat, revisit
   row-template work only
   when coupled to transaction/setup bypass or page-run/record-cell handoff,
   not as another local expression specialization.
4. Treat the 2-writer concurrent row as lower priority until a fresh profile
   shows engine self-time rather than harness/runtime noise.

## Verification

Artifact-only pass:

- No source files edited.
- No negative-results ledger entry added because no source candidate was
  measured and rejected.
- Built benchmark binary with:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-head756-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
- Ran post-landing full quick with:
  `env FSQLITE_BENCH_PROFILE_INSERT=0 FSQLITE_BENCH_PROFILE_DML=0 /data/tmp/frankensqlite-windyibis-head756-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-full-quick.json --no-html`
- Ran focused INSERT profile with:
  `env FSQLITE_BENCH_PROFILE_INSERT=1 FSQLITE_BENCH_PROFILE_DML=0 /data/tmp/frankensqlite-windyibis-head756-target/release-perf/comprehensive-bench --quick --filter INSERT --json-out tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-insert-profile.json --no-html`
- Ran focused DML profile with:
  `env FSQLITE_BENCH_PROFILE_INSERT=0 FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-windyibis-head756-target/release-perf/comprehensive-bench --quick --filter UPDATE --json-out tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-dml-profile.json --no-html`
- CASS index failure recorded above as a blocker for the required session
  refresh.
