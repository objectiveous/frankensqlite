---
bead_id: bd-npn8z
type: forensic-pointer
capture_date: "2026-05-19"
local_path: ci-artifacts/beads-corruption-2026-05-19/
---

# bd-npn8z: Forensic Snapshot Pointer

The full forensic capture is at `ci-artifacts/beads-corruption-2026-05-19/` (gitignored).

## Key Findings

- **100 doubly-referenced B-tree pages** on tree 20 (page 20) — cells 55-80
- **37 rebuild-failed** recovery artifacts since 2026-05-16
- **Corruption mode**: WAL page-pointer clobber on concurrent B-tree interior splits
- **beads.db backup SHA256**: `fbb84fe78b8a9ab9ddcbabdb4182db1f273954db760e838cd2811f12031e180f`

## Artifacts (local only, gitignored)

| File | Size | Description |
|------|------|-------------|
| `br_recovery_rebuild_failed.tar.zst` | 4.3 KB | 37 .rebuild-failed snapshots |
| `forensic-summary.md` | 3.2 KB | Structured frontmatter summary |
| `integrity_check.txt` | 5.1 KB | PRAGMA integrity_check output |
| `dbinfo.txt` | 548 B | SQLite .dbinfo |
| `bak_sha256.txt` | 154 B | SHA256 of 55MB backup |

## Mode Classification

1. WAL page-pointer clobber on concurrent B-tree interior splits (HIGH confidence)
2. Checkpoint-truncation race / bd-yfdb6 (MEDIUM)
3. Non-atomic multi-process `br sync` (MEDIUM)
