# Peer Dirty Lazy DML Fallback Measurement

- Agent: TanBear
- Timestamp: 2026-05-07T18:05Z
- Baseline SHA: `beae52c3` source-equivalent clean benchmark binary from
  `/data/tmp/frankensqlite-small-insert-target/release-perf/comprehensive-bench`
- Candidate: uncommitted peer-owned `crates/fsqlite-core/src/connection.rs`
  changes held by CrimsonGorge, adding lazy VDBE fallback compilation for
  direct-simple prepared UPDATE/DELETE.
- Source ownership: no source files in this bundle were authored, staged, or
  committed by TanBear. This artifact is measurement evidence for the
  peer-owned dirty candidate.

## Candidate Shape Measured

The dirty patch avoids eager fallback bytecode compilation for direct-simple
prepared UPDATE/DELETE statements that normally stay in the direct DML fast
lane. It keeps a placeholder program on the prepared statement, then compiles
the canonical table program only if the direct path falls back.

This targets small UPDATE/DELETE rows where setup and prepared-statement
ceremony dominate the remaining gap versus C SQLite.

## Focused UPDATE/DELETE Gate

Command shape:

```text
<binary> --quick --filter update --json-out <path> --no-html
```

| Metric | Baseline | Candidate |
|---|---:|---:|
| Scenarios | 6 | 6 |
| Franken faster | 0 | 0 |
| Comparable | 1 | 3 |
| C SQLite faster | 5 | 3 |
| Average ratio | 1.314034936395865 | 1.1491932866403862 |
| Geomean ratio | 1.2882128813379434 | 1.1374057797892607 |
| p90 ratio | 1.8378983594191967 | 1.3877829038498963 |
| Weighted score | 1.2882128813379434 | 1.1374057797892607 |

## Full Quick Keep Gate

Command shape:

```text
<binary> --quick --json-out <path> --no-html
```

| Metric | Baseline | Candidate |
|---|---:|---:|
| Scenarios | 93 | 93 |
| Franken faster | 77 | 81 |
| Comparable | 4 | 4 |
| C SQLite faster | 12 | 8 |
| Average ratio | 0.4929414285204568 | 0.45649991120924305 |
| Geomean ratio | 0.2822503300442648 | 0.269163814732017 |
| p90 ratio | 1.0890708559995965 | 0.9992258157875696 |
| p99 ratio | 1.5792735746617377 | 1.326837101301942 |
| Weighted score | 0.36744278283916504 | 0.348430643336749 |

## UPDATE/DELETE Rows In Full Quick

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
|---|---:|---:|---:|---:|
| 100 rows / update 10 rows | 1.5792735746617377 | 1.326837101301942 | 0.129442 | 0.114855 |
| 100 rows / delete 5 rows | 1.4685830641410778 | 1.290885780885781 | 0.116839 | 0.110758 |
| 1000 rows / update 100 rows | 1.122921117426482 | 0.9204863117981887 | 0.427331 | 0.409297 |
| 1000 rows / delete 50 rows | 1.105361242555389 | 0.9639923081816091 | 0.393838 | 0.394028 |
| 10000 rows / update 1000 rows | 0.9880239795732535 | 0.8522945452471847 | 3.445043 | 3.392786 |
| 10000 rows / delete 500 rows | 0.9469194506613924 | 0.8483603495928886 | 3.056406 | 3.134532 |

## Verification

- Dirty candidate build passed:
  `rch exec -- env TMPDIR=/data/tmp/frankensqlite-peer-lazy-dml-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-lazy-dml-target CARGO_BUILD_JOBS=8 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
- Targeted lazy-fallback tests passed remotely:
  `rch exec -- env TMPDIR=/data/tmp/frankensqlite-peer-lazy-dml-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-lazy-dml-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core prepared_update_delete -- --nocapture`
  ran 2 matching tests.
- Targeted direct-simple UPDATE/DELETE tests passed locally:
  `env TMPDIR=/data/tmp/frankensqlite-peer-lazy-dml-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-lazy-dml-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete -- --nocapture`
  ran 2 matching tests.

The remote `test_direct_simple_update_delete` attempt failed with an impossible
partial-sync compile state where the new assertions were visible but the new
`program_is_placeholder` helper was not. The same filter passed locally against
the same dirty source and target, so the failure is recorded as an RCH sync
artifact rather than a candidate correctness failure.

## Interpretation

The candidate clears both the focused DML gate and the full quick keep gate in
this same-window run. It reduces the worst remaining DML ratios and drops full
quick p90 below 1.0. The code is still peer-owned and uncommitted, so the next
step is for CrimsonGorge to either land the patch with this artifact or rerun
the same gates from their workspace before landing.

