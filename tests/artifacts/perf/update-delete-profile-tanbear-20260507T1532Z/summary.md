# update-delete-profile-tanbear-20260507T1532Z

Fresh DML profile on `main` at `4ebec046`, after the insert target pointed
into ledger-fenced standalone ideas.

## Command

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-medium6-profile-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/update-delete-profile-tanbear-20260507T1532Z/update-delete-profile.json \
  --no-html
```

Stdout/stderr:

- `stdout/update-delete-profile.txt`
- `stdout/update-delete-profile.err`

## Section Rows

| Scenario | C SQLite ms | FrankenSQLite ms | Ratio |
| --- | ---: | ---: | ---: |
| `100 rows / update 10 rows` | `0.083937` | `0.130214` | `1.551330164289884` |
| `100 rows / delete 5 rows` | `0.080421` | `0.121007` | `1.5046691784484152` |
| `1000 rows / update 100 rows` | `0.387796` | `0.444162` | `1.1453496168088377` |
| `1000 rows / delete 50 rows` | `0.362099` | `0.408956` | `1.129403837072182` |
| `10000 rows / update 1000 rows` | `3.496179` | `4.072088` | `1.164725261492618` |
| `10000 rows / delete 500 rows` | `3.268572` | `3.765314` | `1.151975235668665` |

## Profile Notes

The direct DML lanes are active (`direct_update` / `direct_delete` match the
mutation counts, `fast` matches the mutation counts, `slow=0`). The isolated
mutation loops are much smaller than the full measured section rows:

- `update_100`: `mutate_us=12.9`, `commit_us=6.3`
- `delete_100`: `mutate_us=8.7`, `commit_us=5.4`
- `update_10000`: `setup_us=2654.7`, `mutate_us=1182.5`, `commit_us=178.3`
- `delete_10000`: `setup_us=2705.1`, `mutate_us=703.2`, `commit_us=152.6`

The small-row DML gaps are therefore dominated by connection/setup,
prepopulation, prepare, and commit ceremony around the tiny mutation core. The
larger rows still inherit much of the insert/prepopulation cost.

## Interpretation

I did not apply a standalone direct-DML patch from this profile. The current
negative-results ledger already fences the obvious local direct-DML retries:
schema-proof carry, scratch-reset removal, same-size REAL leaf patching,
reusable `SharedTxnPageIo`, last-leaf hints, logical DML buffering, and
scan-merge flushing.

The viable retry condition is a true retained-cursor direct-DML kernel or bulk
same-page mutation design that improves the 1K/10K rows and the full
`UPDATE/DELETEThroughput` section in same-window runs.
