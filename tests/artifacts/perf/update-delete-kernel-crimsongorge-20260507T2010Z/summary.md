# Direct DML SharedTxnPageIo context probe rejection

Date: 2026-05-07 20:10Z
Agent: CrimsonGorge
Outcome: rejected and reverted

## Target

The current remaining strict gaps after the direct INSERT layout keep were the
small UPDATE/DELETE rows and the isolated direct-DML mutation kernel. Fresh
isolated baselines showed:

- `10000` rows, isolated `both`: FSQLite `987 ns/update` and `1384 ns/delete`
  versus C SQLite `334 ns/update` and `270 ns/delete`.
- Focused `--quick --filter update` baseline primary score:
  `1.1138800909357498`.

`perf record` samples pointed at `SharedTxnPageIo` page I/O:

- UPDATE: `read_cell_pointers_into`, `table_seek_for_insert`,
  `SharedTxnPageIo::read_page_data`, allocation, and write-page paths.
- DELETE: `TransactionKind::get_page`, `write_page_data`,
  `table_seek_for_insert`, and `SharedTxnPageIo` write paths.

## Candidate

The candidate borrowed `SharedTxnPageIo.concurrent` during hot page I/O instead
of cloning `ConcurrentContext` and its shared handles for each page read/write.
A second variant extended the same shape to page allocation/free and
write-witness hooks.

This preserved the existing concurrent-writer policy and did not change lock
acquisition, FCW checks, read-own-writes behavior, or page-one synthetic commit
surface cleanup semantics.

## Evidence

Focused section:

| Run | Primary score | Avg ratio | Geomean | Result |
| --- | ---: | ---: | ---: | --- |
| Baseline | `1.1138800909357498` | `1.1305813410185228` | `1.1138800909357498` | current |
| Candidate 1 | `1.0819806080388363` | `1.0933894781646951` | `1.0819806080388363` | improved focused row |
| Candidate 2 | `1.1803042031243598` | `1.2063629498128217` | `1.1803042031243598` | rejected |

Isolated `10000` rows:

| Run | F update | F delete | C update | C delete |
| --- | ---: | ---: | ---: | ---: |
| Baseline | `987 ns` | `1384 ns` | `334 ns` | `270 ns` |
| Candidate 1 | `902 ns` | `1322 ns` | `349 ns` | `277 ns` |
| Candidate 2 | `950 ns` | `1383 ns` | `340 ns` | `284 ns` |

Full quick matrix:

| Run | Primary score | Avg ratio | Geomean | Faster / comparable / C faster |
| --- | ---: | ---: | ---: | --- |
| Prior kept full quick | `0.3445386401431955` | `0.45557973340836866` | `0.2635206749084158` | `79 / 5 / 9` |
| Candidate 1 | `0.3447992353725705` | `0.45255878374519515` | `0.2660343910365542` | `82 / 4 / 7` |
| Candidate 1 repeat | `0.3526452109208745` | `0.4827772917582536` | `0.27546142905477156` | `79 / 2 / 12` |
| Candidate 2 | `0.3548895417230123` | `0.46926845894233016` | `0.274253608830876` | `78 / 3 / 12` |

## Decision

Rejected. The narrow variant had a real isolated mutation signal, but the full
matrix did not move in the intended direction. The expanded variant regressed
both the focused section and full quick matrix. The source was manually restored
to the pre-candidate state.

Retry only if paired with a larger direct-DML batching design that amortizes
page I/O setup across many row mutations and proves a full quick matrix win.
