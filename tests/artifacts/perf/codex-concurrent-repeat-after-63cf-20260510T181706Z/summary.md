# Current-Source Concurrent Writer Repeat After `63cfcd95`

Date: 2026-05-10 UTC

Source: `63cfcd959b1145b5005ff48a572fb16b9bf5cb5e`

Command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-after-63cf-current-target \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

/data/tmp/frankensqlite-codex-after-63cf-current-target/release-perf/comprehensive-bench \
  --quick --filter concurrent \
  --json-out tests/artifacts/perf/codex-concurrent-repeat-after-63cf-20260510T181706Z/concurrent.json \
  --no-html
```

Two more same-binary repeats were written to `concurrent-repeat2.json` and
`concurrent-repeat3.json`. A profile-hook run was written to
`concurrent-profile.json` with raw counters in `stderr-profile.log`.

## Repeat Summary

| Run | 2 writers ratio | 4 writers ratio | 8 writers ratio | Geomean |
| --- | ---: | ---: | ---: | ---: |
| `concurrent.json` | 1.10136 | 1.02297 | 0.50409 | 0.82813 |
| `concurrent-repeat2.json` | 1.11099 | 1.00475 | 0.51118 | 0.82943 |
| `concurrent-repeat3.json` | 1.08203 | 1.07921 | 0.49618 | 0.83367 |
| `concurrent-profile.json` | 1.16847 | 1.18188 | 0.48113 | 0.87260 |

Lower is better for FrankenSQLite. The profile-hook run is used for
attribution, not as the clean speed number, because profiling counters add
overhead.

## Profile Hook Attribution

The current 2-writer profile has the expected direct INSERT shape:

- `direct_insert=24012`, `fast=24012`, `slow=0`.
- `page_run_flushes=0`; file-backed concurrent INSERT still does not enter the
  pending page-run path.
- `mvcc_page_lock_waits=12`.
- `mvcc_busy_retries=12`.
- `mvcc_stale_snapshot=12`.
- `mvcc_page_lock_wait_ns=17721247`.

The 4-writer and 8-writer profiles scale the same mechanism:

- 4 writers: `mvcc_page_lock_waits=76`, `mvcc_stale_snapshot=72`.
- 8 writers: `mvcc_page_lock_waits=413`, `mvcc_stale_snapshot=323`.

## Decision

No source patch was kept from this pass. The 2-writer row is a real but narrow
gap: it consistently pays about one transaction-level stale-snapshot retry per
benchmark iteration. The existing transaction retry loop is correct because a
transient error inside `BEGIN CONCURRENT` poisons the whole transaction and
requires rollback before re-BEGIN.

The obvious local alternatives are not valid keep candidates:

- Per-statement retry would retry inside a poisoned transaction.
- Start-gate staggering would change the benchmark's concurrent-writer
  semantics.
- Preemptive file/table-level admission would reintroduce serialized writers,
  which defeats the project invariant.
- Standalone page-run admission is already fenced by prior file-backed
  concurrent artifacts and is still inactive here.

The next credible optimization must change the representation: build and publish
file-backed page batches through MVCC as a unit, or otherwise avoid the
transaction-level stale-snapshot replay without weakening first-committer-wins.
