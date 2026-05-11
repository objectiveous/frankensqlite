# Current DELETE CPU Screen - 2026-05-11

Purpose: current-source delete-body CPU screen after the DML and exact
transaction-control fast-path ledgers fenced the obvious source families.

Source context:

- Git: `main @ 2f8d3b75af4daedbdcd3522c4e599f2694182749`.
- Build: `/data/tmp/frankensqlite-next-profile-target/release-perf/perf-update-delete`.
- Comparison probes:
  - `perf-update-delete 10000 100 delete compare isolated`
  - `perf-update-delete 10000 100 delete compare standard`
- CPU profiles:
  - `perf record -F 997 --call-graph dwarf ... perf-update-delete 10000 2000 delete fsqlite isolated`
  - `perf record --delay 400 -F 997 --call-graph dwarf ... perf-update-delete 10000 4000 delete fsqlite isolated`

Focused timing:

| Probe | FSQLite | C SQLite | F/C ratio |
| --- | ---: | ---: | ---: |
| 10K/500 delete isolated | 357 ns/delete | 273 ns/delete | 1.31x |
| 10K/500 delete standard | 604 ns/delete | 326 ns/delete | 1.85x |

Delete-only delayed perf run:

- Runtime reported by `perf-update-delete`: total `2030 ms`, populate
  `720 ms`, delete `1225 ms`, `613 ns/delete` under profiler overhead.
- `perf record` captured `1752` samples; the committed artifact keeps the
  rendered `perf report --no-children` output in
  `perf-report-delete-only-no-children.txt`.
- Raw `perf.data` captures were intentionally left out of the committed bundle
  after GitHub push protection flagged token-like byte sequences inside the
  binary profiler output. The text reports, stdout/stderr, manifest, and
  checksums are the published evidence.

Top user-space self-time from the delayed report:

| Symbol | Self time |
| --- | ---: |
| `TransactionKind::get_page` | 14.11% |
| `_int_malloc` | 7.68% |
| `TransactionKind::write_page_data` | 6.47% |
| `TableLeafDeleteRun::delete_rowid_with_reason` | 6.03% |
| `TransactionKind::free_page` | 3.99% |
| `__memmove_avx_unaligned_erms` | 3.92% |
| `malloc_consolidate` | 3.12% |
| `CellRef::parse` | 2.73% |
| `TableLeafPayloadPatchRun::table_leaf_rowid_at` | 2.56% |
| freelist serialization/return helpers | about 5.29% combined |

Decision: no source patch from this screen. The profile points back to the
known representation boundary: transaction/page-state work plus same-leaf
delete-run search, not a new isolated hotspot. The standalone source families
that touch these frames have already been measured and rejected unless replaced
by the broader transaction-local DML mutation operator tracked by
`bd-db300.11.1`.
