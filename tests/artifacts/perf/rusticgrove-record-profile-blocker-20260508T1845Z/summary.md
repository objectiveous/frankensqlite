# Record-Size Insert Profile Blocker - 2026-05-08

## Scope

Read-only attribution profile for the remaining record-size direct INSERT
frontier after `2bd0717a982b00bf6216ac70af675ea042de4522`.

This is not a clean keep-gate artifact. The shared checkout had dirty benchmark
methodology edits in:

- `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`
- `crates/fsqlite-e2e/src/bin/perf_update_delete.rs`

Those files were reserved by `SwiftGate` during this pass, so no source edits
were made here. The dirty diff is captured in `dirty-benchmark-files.diff` for
audit.

## Command

```bash
perf record -F 999 -g --call-graph dwarf \
  -o /data/tmp/frankensqlite-rusticgrove-profile-20260508Tnow/record.perf.data \
  -- env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-rusticgrove-current-perf-target/release-perf/comprehensive-bench \
    --quick --filter record \
    --json-out /data/tmp/frankensqlite-rusticgrove-profile-20260508Tnow/record.json \
    --no-html
```

The release-perf target used for the run was no longer present by closeout, so
the raw JSON/stdout/stderr and rendered `perf report --stdio` outputs are the
committed evidence surface. The local ignored `record.perf.data` sidecar was
kept in the workspace but is not part of the committed artifact.

## Results

Record-size summary:

- Scenarios: `4`
- Faster / comparable / slower: `3 / 0 / 1`
- Average ratio: `0.7420956456`
- Geomean ratio: `0.7128730872`
- P90/P99 ratio: `1.0562997650 / 1.0562997650`

Rows:

| Scenario | C SQLite median ms | FrankenSQLite median ms | F/C | C CV | F CV |
| --- | ---: | ---: | ---: | ---: | ---: |
| tiny_1col | 2.316689 | 1.148130 | 0.4956 | 5.9% | 9.0% |
| small_3col | 3.190355 | 2.549885 | 0.7992 | 8.0% | 1.5% |
| medium_6col | 5.739618 | 3.542744 | 0.6172 | 0.4% | 2.1% |
| large_10col | 9.373183 | 9.900891 | 1.0563 | 1.8% | 11.3% |

Representative `large_10col` FrankenSQLite counters from `record.stderr`:

- `setup_us=46.8`, `begin_us=16.5`, `prepare_us=34.9`
- `insert_us=9190.0`, `commit_us=5026.0`
- `row_build_ns=4321656`, `btree_insert_ns=736698`
- `commit_roundtrip_ns=2584791`
- `page_pool_misses=2006`

## Perf Headline

Top user-space samples in `perf-no-children.txt`:

- C SQLite `sqlite3VdbeExec`: `10.86%`
- libc `__memmove_avx_unaligned_erms`: `7.99%`
- `Connection::try_serialize_prepared_direct_simple_insert_record`: `7.22%`
- libc `__memset_avx2_unaligned_erms`: `4.20%`
- C SQLite `sqlite3VdbeMemStringify`: `2.88%`
- `Connection::execute_prepared_direct_simple_insert`: `1.93%`
- `Connection::eval_prepared_direct_simple_insert_expr`: `1.17%`
- `fsqlite_types::serial_type::write_varint`: `0.70%`

The profile does not expose a new unfenced standalone source lever. The visible
cost is the same cluster already covered by the negative ledger: direct record
serialization/row construction, page-copy/zero-fill pressure, and fixed setup.

## Negative-Ledger Fence

Nearby standalone ideas are already rejected in
`docs/progress/perf-negative-results.md`, including:

- benchmark default-page-size PRAGMA elision
- engine-level exact benchmark PRAGMA fast path
- direct INSERT concat record-body encoder
- prepared direct INSERT row-template executor
- direct INSERT fixed cell array staging
- direct serializer one-byte header-size shortcut
- `write_varint` 2/3-byte encoder fast path
- MemoryVfs contiguous batch append
- prebuilt empty-root direct INSERT leaf page-run
- broad page-run/admission threshold changes
- preserialized-record guard widening

## Decision

No source patch was attempted.

This profile should be treated as a blocker/triage artifact: the only slower
record-size row in this run is mild and high-variance, the checkout was dirty in
reserved benchmark files, and the sampled hotspots map to already-fenced
standalone optimization families. A valid next source change needs a
same-window clean A/B that either improves the repeated full quick weighted
score or proves a broader page-builder/row-template fusion that avoids the
fenced micro-optimization shapes.
