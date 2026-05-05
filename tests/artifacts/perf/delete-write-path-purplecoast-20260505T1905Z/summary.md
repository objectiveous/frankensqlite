# Rejected DELETE page-write candidate - 2026-05-05

Agent: PurpleCoast
Purpose: test whether preserving staged-page overwrite steals after transaction-local reads helps the direct DELETE `write_page_data` hotspot.

## Candidate

Changed `SimpleTransaction::get_page` so reads of pages already in the transaction write set returned a transaction-local immutable clone without calling `StagedPage::published_page()`. Also allowed `StagedPageBacking::Owned` to overwrite after its internal immutable snapshot cache had materialized.

Theory: repeated direct DELETE on the same leaf reads a staged page, mutates it, and writes it back. The existing `published_page()` path marks the staged page as published and disables the same-page overwrite steal fast path, so avoiding that mark might reduce `BtCursor::delete -> PageWriter::write_page_data -> TransactionKind::write_page_data`.

## Result

Rejected and reverted. Focused pager staging tests passed, but the isolated DML benchmark regressed.

Baseline from `tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/exact-isolated-compare.log`:

| Metric | Baseline |
| --- | ---: |
| FSQLite total | 580ms |
| FSQLite UPDATE | 263ms |
| FSQLite DELETE | 201ms |
| Total ratio vs C SQLite | 3.20x |
| UPDATE ratio vs C SQLite | 2.75x |
| DELETE ratio vs C SQLite | 5.23x |

Candidate from `candidate-isolated-compare.log`:

| Metric | Candidate |
| --- | ---: |
| FSQLite total | 600ms |
| FSQLite UPDATE | 273ms |
| FSQLite DELETE | 209ms |
| Total ratio vs C SQLite | 3.33x |
| UPDATE ratio vs C SQLite | 2.93x |
| DELETE ratio vs C SQLite | 5.39x |

Do not retry this staged-page publication split without a new profile showing a materially different mechanism.
