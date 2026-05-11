# Direct DELETE Multi-Leaf Batch Full Quick Gate

This run is the full quick keep gate for the final lazy monotone multi-leaf
direct DELETE batching patch.

## Result

- Scenarios: 93
- FSQLite faster / comparable / C SQLite faster: 79 / 1 / 13
- Overall geomean: 0.2797617871920612
- Weighted primary score: 0.3846803250293017
- Previous full quick weighted score: 0.3895403242425907
- Largest DELETE FSQLite median moved from 0.320560 ms to 0.301384 ms.

## Per-Category Geomean

| Category | Rows | Geomean | Average |
| --- | ---: | ---: | ---: |
| concurrent_writers | 3 | 0.8194673994193634 | 0.8643781800452738 |
| mixed | 1 | 0.2026763390984578 | 0.2026763390984578 |
| read_aggregate | 25 | 0.07617444752045649 | 0.2117713192978836 |
| read_single | 33 | 0.214034555953498 | 0.2257966985961791 |
| write_bulk | 22 | 0.8613534768752783 | 0.8921609963453392 |
| write_single | 9 | 1.284540456402075 | 1.4930453812917237 |

## UPDATE/DELETE Rows

| Scenario | Ratio F/C | FSQLite ms | C SQLite ms |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.6202919020715632 | 0.006883 | 0.004248 |
| 100 rows / delete 5 rows | 3.6542783059636994 | 0.008456 | 0.002314 |
| 1000 rows / update 100 rows | 0.8755303322497108 | 0.031779999999999996 | 0.036298 |
| 1000 rows / delete 50 rows | 2.1222778473091366 | 0.033914 | 0.01598 |
| 10000 rows / update 1000 rows | 0.7775054306755789 | 0.278822 | 0.358611 |
| 10000 rows / delete 500 rows | 1.844500477367867 | 0.30138400000000004 | 0.163396 |

The full quick keep gate passes on the primary score and on the target large
DELETE median. Small DELETE remains red and is still a separate DML frontier.
