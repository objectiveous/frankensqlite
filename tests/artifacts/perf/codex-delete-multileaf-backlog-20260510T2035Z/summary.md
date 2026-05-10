# Monotone multi-leaf direct DELETE backlog rejection

Date: 2026-05-10

Decision: rejected and reverted.

## Candidate

The candidate extended the prepared direct DELETE retained leaf-run path in
`crates/fsqlite-core/src/connection.rs` with a backlog of older dirty
`TableLeafDeleteRun`s. A monotone prepared DELETE loop could park a dirty leaf
when it advanced into a later leaf and defer all parked leaves until a read,
backward/duplicate rowid, shape mismatch, or transaction boundary.

The focused correctness proof passed while the candidate was present:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-delete-backlog-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core direct_delete_leaf_run -- --nocapture --test-threads=1
```

## Measurement Integrity Note

The first candidate benchmark files in this directory are invalid for the keep
decision:

- `candidate-update-delete.json`
- `candidate-repeat-update-delete.json`

Those runs used `.rch-target/release-perf/comprehensive-bench`, whose mtime was
`2026-05-08 02:59:19 -0400` even after the candidate build. They also moved C
SQLite medians by about an order of magnitude relative to the parent baseline,
which a FrankenSQLite-only source change cannot explain.

The decision evidence is the clean local rebuild:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-candidate-22361625-local-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Fresh binary:

```text
2026-05-10 16:58:58 -0400 15970968 /data/tmp/frankensqlite-candidate-22361625-local-target/release-perf/comprehensive-bench
```

## Focused A/B

Parent baseline repeat:

```text
baseline-repeat-update-delete.json
geomean 1.6856942137
p90     3.5704903678
```

Clean candidate repeat:

```text
candidate-local-repeat-update-delete.json
geomean 1.6093110443
p90     3.5543340381
```

Key DELETE rows:

```text
100 rows / delete 5 rows:
  baseline repeat 3.5704903678x, FSQLite 0.008155 ms
  candidate repeat 3.5543340381x, FSQLite 0.008406 ms

1000 rows / delete 50 rows:
  baseline repeat 2.0936538462x, FSQLite 0.032661 ms
  candidate repeat 2.1191039203x, FSQLite 0.034054 ms

10000 rows / delete 500 rows:
  baseline repeat 1.8770632596x, FSQLite 0.319097 ms
  candidate repeat 2.1230498875x, FSQLite 0.335858 ms
```

The C SQLite medians in the clean candidate run stayed in the same range as the
baseline, so the rejection is based on comparable measurements. The simple
backlog did not improve the target DELETE rows and slightly worsened the large
DELETE row.
