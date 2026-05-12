# Current-HEAD INSERT Boundary Refresh

Date: 2026-05-12

Commit under review: `369b3052e08c3744bc2329b395afe7a1cc5ee09d`

## Command

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-codex-current-dml-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-current-head-insert-profile-20260512T0218/insert-profile.json \
  --no-html \
  2>&1 | tee tests/artifacts/perf/codex-current-head-insert-profile-20260512T0218/stdout.txt
```

The benchmark binary warning is expected for this profile: the intervening HEAD
commits were docs/artifact commits after the release-perf binary was built, not
Rust source changes.

## Focused Profile

The focused INSERT run reported 25 scenarios:

- FSQLite faster / comparable / C SQLite faster: `14 / 7 / 4`.
- Average F/C ratio: `0.8367223988`.
- Geomean F/C ratio: `0.7919573646`.
- Median F/C ratio: `0.8687901212`.
- P90 / P99 F/C ratio: `1.1095440495 / 1.3783689373`.
- Focused weighted score: `0.8665744989`.

Rows with raw F/C ratio above `1.0` were:

- `small_3col` 100 rows: C `0.074430 ms`, F `0.102592 ms`, ratio `1.378x`.
- `medium_6col` 100 rows: C `0.139752 ms`, F `0.155061 ms`, ratio `1.110x`.
- `large_10col` 100 rows: C `0.244288 ms`, F `0.249758 ms`, ratio `1.022x`.
- `small_3col` 100 rows / batched: C `0.117951 ms`, F `0.123992 ms`, ratio `1.051x`.
- `small_3col` 100 rows / single txn: C `0.119764 ms`, F `0.121358 ms`, ratio `1.013x`.
- `small_3col` 10000 rows / autocommit: C `11.336422 ms`, F `11.541306 ms`, ratio `1.018x`.
- `small_3col` 10000 rows / single txn: C `3.767036 ms`, F `3.904653 ms`, ratio `1.037x`.
- Record-size `small_3col` 10K: C `3.555389 ms`, F `3.953645 ms`, ratio `1.112x`.

The focused repeat flipped the prior `large_10col` 10K concern green: single
transaction `large_10col` 10K was C `14.32 ms`, F `14.14 ms`, and record-size
`large_10col` 10K was C `17.79 ms`, F `17.29 ms`.

## Source Check

The direct INSERT code reread did not expose a new narrow source lever:

- `try_serialize_prepared_direct_simple_insert_record` is already the active
  path for the profiled rows (`direct_insert == fast`, `slow=0`).
- The 100-row rows are fixed-cost dominated, with row construction in the tens
  of microseconds and B-tree work in the low single-digit microseconds.
- The remaining 10K `small_3col` rows still point at the preserialized
  row/body construction path, not B-tree insertion.
- The obvious source families here overlap the already-fenced INSERT
  serializer, concat/param-one/template, row-scratch, page-run threshold/arena,
  prebuilt page image, and borrowed flush attempts.

## Outcome

No source patch was attempted. This artifact records the current INSERT
boundary after the current fullquick refresh: the remaining INSERT work is now
small-row fixed cost and row/body construction, while the prior large-row 10K
INSERT concern is not stable in the focused profile.
