# medium6-single-insert-profile-tanbear-20260507T1522Z

Fresh profile on `main` at `4ebec046` after the depth-2 right-edge page-run
bulk append landed.

## Command

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-medium6-profile-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/medium6-single-insert-profile-tanbear-20260507T1522Z/insert-profile.json \
  --no-html
```

Stdout/stderr:

- `stdout/insert-profile.txt`
- `stdout/insert-profile.err`

## Target Row

`INSERTThroughput - Single Transaction - medium_6col`, `1000 rows`:

- C SQLite median: `0.537196 ms`
- FrankenSQLite median: `0.806150 ms`
- Ratio: `1.500662700392408`

Profile line:

- `setup_us=20.4`
- `begin_us=11.0`
- `prepare_us=13.1`
- `insert_us=801.2`
- `commit_us=269.5`
- `row_build_ns=286119`
- `btree_insert_ns=117822`
- `memdb_apply_ns=24069`
- `schema_validation_ns=32054`
- `change_tracking_ns=23793`
- `page_pool_misses=48`

The same section's `10000 rows` medium case was `1.2361664904987206x`
slower than C SQLite, with `row_build_ns=3075041`, `btree_insert_ns=1737126`,
and `commit_us=2871.0`.

## Interpretation

The remaining medium-row insert gap is not in the already-landed non-empty
right-edge append path. For empty-root single-transaction inserts, the profile
still points at row expression/record construction plus deferred page-run
materialization at commit.

I did not apply a standalone source patch from this profile because the
current negative-results ledger already fences the obvious local retries:

- direct INSERT row-build arithmetic/template specialization
- concat text-piece collection
- empty-root bulk-loader root-fit prechecks
- duplicate leaf-group reuse
- direct pointer writes in the bulk page builders
- count/sum and quotient-filter maintenance skips

The viable retry condition from the ledger is narrower: a fused
record/page-run builder that constructs records and page layout together over a
many-row run, then proves same-window insert and full quick matrix wins.
