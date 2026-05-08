# File-backed direct-record preserialize probe

Candidate: allow the prepared-direct INSERT borrowed-parameter record serializer
on file-backed explicit transactions when the MemDB mirror is already
unloaded/stale (`in_transaction && !track_memdb_delta`). Source was reverted
after the comprehensive concurrent gate rejected it.

## Focused `mt-mvcc-bench`, 2 writers x 1000 rows

Same-window 30-iteration A/B:

| Build | FSQLite p50 ms | C SQLite p50 ms | Time ratio | Throughput ratio |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 4.04 | 2.34 | 1.72x | 0.58x |
| Candidate | 3.95 | 2.39 | 1.65x | 0.60x |

The standalone row improved slightly, so the candidate advanced to the
comprehensive concurrent filter.

## `comprehensive-bench --quick --filter concurrent`

| Row | Baseline F ms | Candidate F ms | Baseline ratio | Candidate ratio |
| --- | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 14.408550 | 13.897172 | 1.058592 | 1.117501 |
| 4 writers x 1000 rows | 20.472354 | 20.498083 | 0.974043 | 1.030069 |
| 8 writers x 1000 rows | 35.757045 | 44.320340 | 0.390803 | 0.483377 |

Result: reject. The candidate's small 2-writer absolute improvement was
outweighed by worse C-relative ratios and a large 8-writer FSQLite regression.

Artifacts:

- `baseline-mt-2t-30.json`
- `candidate-mt-2t-30.json`
- `baseline-concurrent.json`
- `candidate-concurrent.json`
