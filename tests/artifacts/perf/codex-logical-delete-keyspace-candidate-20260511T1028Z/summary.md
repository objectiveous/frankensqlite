# Prepared direct DELETE rowid-keyspace candidate

Date: 2026-05-11.

Purpose: screen a prepared-statement rowid keyspace for direct DELETE. The
candidate seeded the keyspace before the measured loop, maintained it through
direct INSERT restore/delete operations, returned affected counts from that
transaction-local view, and deferred physical B-tree DELETE publication until
the normal pending direct-write flush boundary.

Command:

```bash
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter delete \
  --json-out tests/artifacts/perf/codex-logical-delete-keyspace-candidate-20260511T1028Z/update-delete-quick.json \
  --no-html
```

Focused quick result:

| Scenario | C SQLite ms | FrankenSQLite ms | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 0.002815 | 0.008195 | 2.911x |
| 1000 rows / delete 50 rows | 0.015830 | 0.032871 | 2.077x |
| 10000 rows / delete 500 rows | 0.161312 | 0.300753 | 1.864x |

Rejected: the target FSQLite DELETE medians stayed in the same noise band as
the current baseline (`0.008456`, `0.033914`, `0.301384` ms for the same rows)
and did not move the section outcome. The source patch and focused test were
manually unwound.

Checksums:

```text
cb53ca9a6d6cab3747737335bd83303d6fec1c67c854dac8e3d5d1debb89b531  stderr.txt
ce0ec0a43dca52f885aedbc3a55036fea7e43fca1f36614d293c226404a90eb6  stdout.txt
e63f3ccc9471101292db5cb6a4acfdebad840f70ac5d510d916aa8c7b755326c  update-delete-quick.json
```
