# Current HEAD DML Profile

This run is a focused rebuilt current-HEAD `--quick --filter update` benchmark
with `FSQLITE_BENCH_PROFILE_DML=1` enabled to attribute the remaining
UPDATE/DELETE rows.

## Source

- Source commit reported by benchmark:
  `fa0f3073da5866f409ff3053263043e5e58e3b97`
- Build command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-head-local-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
- Run command:
  `FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-current-head-local-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/codex-current-head-dml-profile-20260511T043335Z/update-delete.json --no-html`

The benchmark stdout reports `Git dirty: yes` because the checkout has
pre-existing untracked `.rch-*` target directories. No source changes were
present in this run.

## Result

- Scenarios: 6
- FSQLite faster / comparable / C SQLite faster: 1 / 0 / 5
- Average ratio: 2.326872848312272
- Geomean ratio: 1.8774822982380694
- Median ratio: 1.8675914886100544
- P90 ratio: 6.161331086773378
- P99 ratio: 6.161331086773378
- Primary weighted score: 1.8774822982380694

## Rows

| Scenario | Ratio F/C | FSQLite ms | C SQLite ms | Category |
| --- | ---: | ---: | ---: | --- |
| 100 rows / update 10 rows | 1.6023954908407703 | 0.006823 | 0.004258 | write_single |
| 100 rows / delete 5 rows | 6.161331086773378 | 0.014627000000000001 | 0.0023740000000000002 | write_single |
| 1000 rows / update 100 rows | 1.5055177336115075 | 0.054843 | 0.036427999999999995 | write_single |
| 1000 rows / delete 50 rows | 2.057601977750309 | 0.033292 | 0.01618 | write_single |
| 10000 rows / update 1000 rows | 0.7667993122876123 | 0.284994 | 0.371667 | write_single |
| 10000 rows / delete 500 rows | 1.8675914886100544 | 0.296045 | 0.158517 | write_single |

## Profile Takeaways

The high-variance `100 rows / delete 5 rows` row is too noisy to optimize from
alone, but the profile still points at the same source frontier as the full
quick matrix: explicit-transaction DELETE overhead is concentrated in
transaction-local direct-delete mutation and pending leaf-run flush/materialize
work, not SQL parsing, prepared lookup, background checks, or pager commit.

For the larger DELETE rows, the measured subphase costs were:

| Scenario | mutate us | commit us | leaf active | leaf misses | leaf flush ns | materialize ns | write ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1000 rows / delete 50 rows | 22.7 | 23.3 | 44/49 | 5 | 13746 | 10007 | 2797 |
| 10000 rows / delete 500 rows | 314.1 | 46.0 | 433/496 | 63 | 109516 | 78149 | 24061 |

Prior ledger entries reject standalone retained DELETE leaf-run tweaks, global
leaf-run disabling, direct-flush wrappers, transaction-control
parser/background trimming, prepared-cache last-hit changes, and root-`Cx`
reuse. A keep-worthy DML source change now needs to avoid those fenced families;
the remaining credible candidate is a broader transaction-local DML mutation
operator with read-your-writes, rollback/savepoint, failed-flush preservation,
quotient-filter/cache invalidation, schema-drift, and MVCC-publication proof.
