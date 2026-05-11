# Prepared direct DELETE logical buffer candidate

Date: 2026-05-11.

Purpose: screen a transaction-local prepared direct DELETE buffer that returned
affected counts from the exact private `:memory:` MemDatabase mirror and deferred
physical B-tree publication until the normal pending direct-write flush boundary.

Commands:

```bash
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter delete \
  --json-out tests/artifacts/perf/codex-logical-delete-candidate-20260511T1019Z/update-delete-quick.json \
  --no-html

env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-codex-logical-delete-bench-target/release-perf/comprehensive-bench \
  --quick --filter delete \
  --json-out tests/artifacts/perf/codex-logical-delete-candidate-20260511T1019Z/update-delete-profile-quick.json \
  --no-html
```

Focused quick result:

| Scenario | C SQLite ms | FrankenSQLite ms | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 0.002305 | 0.008265 | 3.586x |
| 1000 rows / delete 50 rows | 0.015919 | 0.033052 | 2.076x |
| 10000 rows / delete 500 rows | 0.161132 | 0.296605 | 1.841x |

Profile-enabled focused result:

| Scenario | C SQLite ms | FrankenSQLite ms | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 0.002315 | 0.008486 | 3.666x |
| 1000 rows / delete 50 rows | 0.016020 | 0.033333 | 2.081x |
| 10000 rows / delete 500 rows | 0.160651 | 0.293519 | 1.827x |

Rejected: the MemDatabase-only logical path did not activate in the benchmark
shape after populate/restore teardown. The profile still reported the existing
physical retained DELETE path, including `delete_seek_ns`, `delete_leaf_active`,
and `delete_leaf_flush` counters.

Checksums:

```text
9dbdf7f0bc0e7b2c8a9d3e1b456de3e0954989b49511a44870775c573b00968a  profile-stderr.txt
8361923981bd7ab73673d738ec31adb79537685b6e6f5b71d81a25489696cd35  profile-stdout.txt
3febb7bee831a7af3a7f82caf0c0752d95464c1ad237bf19d86a733ee62e6186  stderr.txt
8a0473333cf7c903423f7a2e74cd258b4818e209bcc5f39323e242cfe414aae3  stdout.txt
5c60293269f692eeb0919119c706c2beca58e11bef1057f871eba34ead03beb4  update-delete-profile-quick.json
f3760b3514be86156cdec7edfa56305684d2ec1413549e8f10bb1f4c068ad9bc  update-delete-quick.json
```
