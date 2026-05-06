//! Durable file-backed time-travel history sidecar.
//!
//! This module persists the per-commit MemDatabase snapshots that back
//! `FOR SYSTEM_TIME AS OF COMMITSEQ N` for file-backed Native-mode databases
//! (issue #82).
//!
//! ## On-disk format
//!
//! The history is stored in a sidecar file named `<db_path>-fshistory`
//! alongside the main database file (and the WAL). The format is
//! deliberately simple, append-only, and self-describing so that:
//!
//! 1. A crash mid-write can be recovered by truncating the partial trailing
//!    record. The live database file and WAL are completely untouched —
//!    the history sidecar is purely advisory.
//! 2. Old records can be garbage-collected by rewriting the file in place
//!    (atomic rename of `<path>-fshistory.compact`).
//!
//! Layout:
//!
//! ```text
//!   ┌────────────────────────────────┐
//!   │ FSHX magic + format version    │  16 bytes
//!   ├────────────────────────────────┤
//!   │ record 0 frame                 │  variable
//!   ├────────────────────────────────┤
//!   │ record 1 frame                 │  variable
//!   ├────────────────────────────────┤
//!   │ ...                            │
//!   └────────────────────────────────┘
//! ```
//!
//! Each **record frame** is:
//!
//! ```text
//!   u32 LE  payload_len  (bytes in payload below)
//!   u64 LE  commit_seq
//!   u64 LE  timestamp_ns
//!   u64 LE  payload_xxh3   (xxhash-3 of payload)
//!   bytes   payload (length = payload_len, JSON-encoded SnapshotPayload)
//! ```
//!
//! The xxh3 checksum lets the read path detect torn writes (partial flush,
//! disk corruption) and stop reading at the last well-formed record without
//! corrupting the main DB.
//!
//! The payload uses serde_json because:
//! - `SqliteValue` already derives `Serialize`/`Deserialize`
//! - human-readable for inspection / debugging in the field
//! - file size is bounded by the retention policy, not throughput-critical
//!
//! ## Concurrency / durability
//!
//! - All writes happen on the connection thread that finalizes a commit,
//!   under the same logical critical section as the in-memory snapshot
//!   capture. There is no cross-process MVCC for time-travel history yet
//!   (deliberate non-goal per #82's minimal-subset framing).
//! - We `flush()` after each record so that a process crash loses at most
//!   the current in-flight record. We do **not** call `sync_all()` per
//!   record (that would dominate commit latency); the history sidecar is
//!   advisory and a missed sync just trims the most recent commits from
//!   the readable history. Sync happens on connection close and on GC
//!   compaction.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fsqlite_error::{FrankenError, Result};
use fsqlite_types::value::SqliteValue;
use fsqlite_vdbe::engine::MemDatabase;
use serde::{Deserialize, Serialize};

/// Magic bytes that identify a FrankenSQLite history sidecar file.
pub(crate) const HISTORY_MAGIC: [u8; 8] = *b"FSHX-v01";
/// File header length: magic (8) + format version (u32 LE) + reserved (u32 LE).
pub(crate) const HISTORY_HEADER_LEN: u64 = 16;
/// Record frame fixed prefix length: payload_len (4) + commit_seq (8)
/// + timestamp_ns (8) + payload_xxh3 (8) = 28 bytes.
pub(crate) const RECORD_FRAME_PREFIX_LEN: usize = 28;
/// Current on-disk format version. Bump when the payload schema changes
/// in an incompatible way; readers refuse unknown versions.
pub(crate) const HISTORY_FORMAT_VERSION: u32 = 1;

/// Default retention policy: keep the most recent 1000 snapshots.
pub const DEFAULT_RETENTION_LIMIT: usize = 1000;

/// JSON-encoded record payload for a single committed snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotPayload {
    /// `MemDatabase::next_root_page` at capture time. Required so that the
    /// rebuilt `MemDatabase` allocates fresh root pages above the live
    /// schema's high-water mark when historical reads materialize new
    /// implicit cursors.
    pub next_root_page: i32,
    /// One entry per logical table tracked at capture time.
    pub tables: Vec<SnapshotTable>,
}

/// JSON-encoded representation of a single `MemTable`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotTable {
    pub root_page: i32,
    pub num_columns: usize,
    pub rows: Vec<SnapshotRow>,
}

/// JSON-encoded representation of a single row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotRow {
    pub rowid: i64,
    pub values: Vec<SqliteValue>,
}

/// One entry parsed from the sidecar file.
#[derive(Debug, Clone)]
pub struct LoadedSnapshot {
    pub commit_seq: u64,
    pub timestamp_ns: u64,
    pub db: MemDatabase,
}

/// Returns the sidecar history path corresponding to a database path.
///
/// `:memory:` databases have no sidecar (returns `None`). Empty paths
/// also return `None` defensively.
#[must_use]
pub fn history_path_for(db_path: &str) -> Option<PathBuf> {
    if db_path.is_empty() || db_path == ":memory:" {
        return None;
    }
    let mut sidecar = PathBuf::from(db_path);
    let file_name = sidecar.file_name().map(|n| n.to_owned())?;
    let mut new_name = file_name;
    new_name.push("-fshistory");
    sidecar.set_file_name(new_name);
    Some(sidecar)
}

/// Compaction sidecar path (used during GC).
fn compact_path_for(history: &Path) -> PathBuf {
    let mut p = history.as_os_str().to_owned();
    p.push(".compact");
    PathBuf::from(p)
}

fn xxh3_64(bytes: &[u8]) -> u64 {
    xxhash_rust::xxh3::xxh3_64(bytes)
}

/// Build a `SnapshotPayload` from the live `MemDatabase`. Only tables
/// reachable through the schema vector are recorded; sentinel/scratch
/// root-pages allocated by transient cursors are intentionally skipped so
/// we don't pay quadratic clones on transient state.
pub(crate) fn snapshot_payload_from_memdb(
    db: &MemDatabase,
    schema_root_pages: &[i32],
) -> SnapshotPayload {
    let mut tables = Vec::with_capacity(schema_root_pages.len());
    for &root_page in schema_root_pages {
        if root_page <= 0 {
            continue;
        }
        let Some(mem_table) = db.get_table(root_page) else {
            continue;
        };
        let mut rows = Vec::with_capacity(mem_table.row_count());
        for (rowid, values) in mem_table.iter_rows() {
            rows.push(SnapshotRow {
                rowid,
                values: values.to_vec(),
            });
        }
        tables.push(SnapshotTable {
            root_page,
            num_columns: mem_table.num_columns,
            rows,
        });
    }
    SnapshotPayload {
        next_root_page: db.next_root_page(),
        tables,
    }
}

/// Materialize a `MemDatabase` from a `SnapshotPayload`. Tables are
/// re-created at their original root pages and populated with the captured
/// rowid/value pairs.
pub(crate) fn memdb_from_snapshot_payload(payload: &SnapshotPayload) -> MemDatabase {
    let mut db = MemDatabase::new();
    db.set_next_root_page(payload.next_root_page);
    for table in &payload.tables {
        db.create_table_at(table.root_page, table.num_columns);
        if let Some(mem_table) = db.get_table_mut(table.root_page) {
            for row in &table.rows {
                mem_table.insert_row(row.rowid, row.values.clone());
            }
        }
    }
    db
}

/// Write-side handle for the durable history sidecar.
#[derive(Debug)]
pub struct HistoryWriter {
    path: PathBuf,
    file: File,
}

impl HistoryWriter {
    /// Open (and if necessary create) the sidecar history file in append mode.
    ///
    /// On a brand-new file the header is written immediately; on an existing
    /// file the header is validated and the cursor is positioned at EOF
    /// after the last well-formed record (truncating any torn trailing
    /// record).
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|err| {
                FrankenError::Internal(format!(
                    "time-travel history: failed to open {} ({err})",
                    path.display()
                ))
            })?;

        let len = file
            .metadata()
            .map_err(|err| {
                FrankenError::Internal(format!(
                    "time-travel history: stat failed for {} ({err})",
                    path.display()
                ))
            })?
            .len();

        if len == 0 {
            write_header(&mut file)?;
        } else {
            let truncate_to = validate_header_and_scan(&mut file, len)?;
            if truncate_to < len {
                tracing::warn!(
                    target: "fsqlite.time_travel",
                    path = %path.display(),
                    file_len = len,
                    truncate_to,
                    "trimming torn trailing record from time-travel history"
                );
                file.set_len(truncate_to).map_err(|err| {
                    FrankenError::Internal(format!(
                        "time-travel history: truncate failed for {} ({err})",
                        path.display()
                    ))
                })?;
            }
            file.seek(SeekFrom::End(0)).map_err(|err| {
                FrankenError::Internal(format!(
                    "time-travel history: seek-end failed for {} ({err})",
                    path.display()
                ))
            })?;
        }

        Ok(Self {
            path: path.to_path_buf(),
            file,
        })
    }

    /// Append a new snapshot record to the sidecar.
    pub(crate) fn append_snapshot(
        &mut self,
        commit_seq: u64,
        timestamp_ns: u64,
        payload: &SnapshotPayload,
    ) -> Result<()> {
        let json = serde_json::to_vec(payload).map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: serialize payload failed ({err})"
            ))
        })?;
        let payload_len: u32 = u32::try_from(json.len()).map_err(|_| {
            FrankenError::Internal("time-travel history: payload exceeds u32 byte limit".to_owned())
        })?;
        let xxh = xxh3_64(&json);

        let mut frame = Vec::with_capacity(RECORD_FRAME_PREFIX_LEN + json.len());
        frame.extend_from_slice(&payload_len.to_le_bytes());
        frame.extend_from_slice(&commit_seq.to_le_bytes());
        frame.extend_from_slice(&timestamp_ns.to_le_bytes());
        frame.extend_from_slice(&xxh.to_le_bytes());
        frame.extend_from_slice(&json);

        self.file.write_all(&frame).map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: write failed for {} ({err})",
                self.path.display()
            ))
        })?;
        // We intentionally only flush — not fsync — on the hot commit path.
        // See module docs.
        self.file.flush().map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: flush failed for {} ({err})",
                self.path.display()
            ))
        })?;
        Ok(())
    }

    /// Force-sync the sidecar to disk. Called on connection close and after GC.
    pub fn sync(&mut self) -> Result<()> {
        self.file.sync_data().map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: fsync failed for {} ({err})",
                self.path.display()
            ))
        })
    }

    /// Compact the history file by retaining only the last `keep` records.
    ///
    /// The compaction is performed by writing to a sibling `.compact` file
    /// then atomically renaming over the original. On failure the original
    /// file is preserved.
    pub fn compact(&mut self, keep: usize) -> Result<usize> {
        if keep == 0 {
            return Ok(0);
        }
        // Read all records from the current file, drop oldest until <= keep.
        let snapshots = read_all_records(&self.path)?;
        if snapshots.len() <= keep {
            return Ok(snapshots.len());
        }
        let drop_count = snapshots.len() - keep;
        let retained = &snapshots[drop_count..];

        let compact_path = compact_path_for(&self.path);
        // Truncate any leftover compact file from a previous failed run.
        let _ = std::fs::remove_file(&compact_path);
        {
            let mut compact_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&compact_path)
                .map_err(|err| {
                    FrankenError::Internal(format!(
                        "time-travel history: compact create failed for {} ({err})",
                        compact_path.display()
                    ))
                })?;
            write_header(&mut compact_file)?;
            let mut writer = Self {
                path: compact_path.clone(),
                file: compact_file,
            };
            for snap in retained {
                let payload = snapshot_payload_from_memdb(&snap.db, &collect_root_pages(&snap.db));
                writer.append_snapshot(snap.commit_seq, snap.timestamp_ns, &payload)?;
            }
            writer.sync()?;
        }
        // Atomic rename over the live sidecar.
        std::fs::rename(&compact_path, &self.path).map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: rename {} -> {} failed ({err})",
                compact_path.display(),
                self.path.display()
            ))
        })?;
        // Re-open for further appends.
        let new_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                FrankenError::Internal(format!(
                    "time-travel history: reopen after compact failed for {} ({err})",
                    self.path.display()
                ))
            })?;
        self.file = new_file;
        self.file.seek(SeekFrom::End(0)).map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: seek-end after compact failed for {} ({err})",
                self.path.display()
            ))
        })?;
        Ok(retained.len())
    }
}

/// Read every well-formed record from the sidecar at `path`. Returns an
/// empty vec if the file does not exist.
pub fn read_all_records(path: &Path) -> Result<Vec<LoadedSnapshot>> {
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(FrankenError::Internal(format!(
                "time-travel history: open-read failed for {} ({err})",
                path.display()
            )));
        }
    };
    let len = file
        .metadata()
        .map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: stat failed for {} ({err})",
                path.display()
            ))
        })?
        .len();
    if len == 0 {
        return Ok(Vec::new());
    }

    let mut header = [0u8; HISTORY_HEADER_LEN as usize];
    file.read_exact(&mut header).map_err(|err| {
        FrankenError::Internal(format!(
            "time-travel history: header read failed for {} ({err})",
            path.display()
        ))
    })?;
    validate_header_bytes(&header)?;

    let mut records = Vec::new();
    let mut cursor: u64 = HISTORY_HEADER_LEN;
    while cursor < len {
        let Some(record) = read_one_record(&mut file, cursor, len)? else {
            // Torn trailing record: stop, leave the rest to writer-side recovery.
            tracing::warn!(
                target: "fsqlite.time_travel",
                path = %path.display(),
                file_len = len,
                cursor,
                "stopped reading time-travel history at torn record"
            );
            break;
        };
        cursor = record.next_cursor;
        let payload: SnapshotPayload = serde_json::from_slice(&record.payload).map_err(|err| {
            FrankenError::Internal(format!(
                "time-travel history: payload decode failed at offset {} ({err})",
                record.frame_offset
            ))
        })?;
        let db = memdb_from_snapshot_payload(&payload);
        records.push(LoadedSnapshot {
            commit_seq: record.commit_seq,
            timestamp_ns: record.timestamp_ns,
            db,
        });
    }
    Ok(records)
}

fn write_header(file: &mut File) -> Result<()> {
    let mut buf = [0u8; HISTORY_HEADER_LEN as usize];
    buf[..8].copy_from_slice(&HISTORY_MAGIC);
    buf[8..12].copy_from_slice(&HISTORY_FORMAT_VERSION.to_le_bytes());
    // Reserved bytes (4): zeros today, room to extend later (e.g. flags).
    file.write_all(&buf).map_err(|err| {
        FrankenError::Internal(format!("time-travel history: header write failed ({err})"))
    })?;
    file.flush().map_err(|err| {
        FrankenError::Internal(format!("time-travel history: header flush failed ({err})"))
    })
}

fn validate_header_bytes(bytes: &[u8]) -> Result<()> {
    if bytes.len() < HISTORY_HEADER_LEN as usize {
        return Err(FrankenError::Internal(
            "time-travel history: header truncated".to_owned(),
        ));
    }
    if bytes[..8] != HISTORY_MAGIC {
        return Err(FrankenError::Internal(
            "time-travel history: bad magic (not a FrankenSQLite history sidecar)".to_owned(),
        ));
    }
    let version = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    if version != HISTORY_FORMAT_VERSION {
        return Err(FrankenError::Internal(format!(
            "time-travel history: unsupported format version {version} \
             (this build supports {HISTORY_FORMAT_VERSION})"
        )));
    }
    Ok(())
}

fn validate_header_and_scan(file: &mut File, len: u64) -> Result<u64> {
    if len < HISTORY_HEADER_LEN {
        return Err(FrankenError::Internal(
            "time-travel history: file shorter than header".to_owned(),
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|err| {
        FrankenError::Internal(format!("time-travel history: seek-start failed ({err})"))
    })?;
    let mut header = [0u8; HISTORY_HEADER_LEN as usize];
    file.read_exact(&mut header).map_err(|err| {
        FrankenError::Internal(format!("time-travel history: header read failed ({err})"))
    })?;
    validate_header_bytes(&header)?;

    let mut cursor: u64 = HISTORY_HEADER_LEN;
    let mut last_good: u64 = HISTORY_HEADER_LEN;
    while cursor < len {
        let Some(record) = read_one_record(file, cursor, len)? else {
            // Stop scanning at the first torn record — its tail will be truncated.
            break;
        };
        last_good = record.next_cursor;
        cursor = record.next_cursor;
    }
    Ok(last_good)
}

struct ReadRecord {
    payload: Vec<u8>,
    commit_seq: u64,
    timestamp_ns: u64,
    frame_offset: u64,
    next_cursor: u64,
}

fn read_one_record(file: &mut File, offset: u64, file_len: u64) -> Result<Option<ReadRecord>> {
    if offset.saturating_add(RECORD_FRAME_PREFIX_LEN as u64) > file_len {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset)).map_err(|err| {
        FrankenError::Internal(format!(
            "time-travel history: seek failed at offset {offset} ({err})"
        ))
    })?;
    let mut prefix = [0u8; RECORD_FRAME_PREFIX_LEN];
    file.read_exact(&mut prefix).map_err(|err| {
        FrankenError::Internal(format!(
            "time-travel history: prefix read failed at offset {offset} ({err})"
        ))
    })?;
    let payload_len = u32::from_le_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]);
    let commit_seq = u64::from_le_bytes([
        prefix[4], prefix[5], prefix[6], prefix[7], prefix[8], prefix[9], prefix[10], prefix[11],
    ]);
    let timestamp_ns = u64::from_le_bytes([
        prefix[12], prefix[13], prefix[14], prefix[15], prefix[16], prefix[17], prefix[18],
        prefix[19],
    ]);
    let expected_xxh = u64::from_le_bytes([
        prefix[20], prefix[21], prefix[22], prefix[23], prefix[24], prefix[25], prefix[26],
        prefix[27],
    ]);
    let body_end = offset
        .saturating_add(RECORD_FRAME_PREFIX_LEN as u64)
        .saturating_add(u64::from(payload_len));
    if body_end > file_len {
        return Ok(None);
    }
    let mut payload = vec![0u8; payload_len as usize];
    file.read_exact(&mut payload).map_err(|err| {
        FrankenError::Internal(format!(
            "time-travel history: payload read failed at offset {offset} ({err})"
        ))
    })?;
    let actual_xxh = xxh3_64(&payload);
    if actual_xxh != expected_xxh {
        // Treat mismatched checksum as a torn record — stop here.
        tracing::warn!(
            target: "fsqlite.time_travel",
            offset,
            expected_xxh,
            actual_xxh,
            "time-travel history record checksum mismatch; treating as torn write"
        );
        return Ok(None);
    }
    Ok(Some(ReadRecord {
        payload,
        commit_seq,
        timestamp_ns,
        frame_offset: offset,
        next_cursor: body_end,
    }))
}

/// Collect the root pages from a `MemDatabase`'s tables map. Used during
/// compaction when we re-serialize the loaded snapshots.
fn collect_root_pages(db: &MemDatabase) -> Vec<i32> {
    let mut roots: Vec<i32> = db.tables.iter().map(|(root, _)| *root).collect();
    roots.sort_unstable();
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_db(rows: &[(i32, i64, &str)]) -> MemDatabase {
        let mut db = MemDatabase::new();
        // Page 1 reserved for sqlite_master, real tables start at 2.
        for &(root, rowid, text) in rows {
            if db.get_table(root).is_none() {
                db.create_table_at(root, 1);
            }
            if let Some(table) = db.get_table_mut(root) {
                table.insert_row(rowid, vec![SqliteValue::Text(text.into())]);
            }
        }
        db
    }

    #[test]
    fn history_path_for_handles_memory_and_paths() {
        assert!(history_path_for(":memory:").is_none());
        assert!(history_path_for("").is_none());
        let p = history_path_for("/tmp/foo.db").unwrap();
        assert_eq!(p.file_name().unwrap(), "foo.db-fshistory");
    }

    #[test]
    fn append_then_read_back_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hist.db-fshistory");
        let db1 = make_test_db(&[(2, 1, "alpha")]);
        let db2 = make_test_db(&[(2, 1, "alpha"), (2, 2, "beta")]);
        {
            let mut writer = HistoryWriter::open(&path).unwrap();
            let payload1 = snapshot_payload_from_memdb(&db1, &[2]);
            let payload2 = snapshot_payload_from_memdb(&db2, &[2]);
            writer.append_snapshot(1, 1_000, &payload1).unwrap();
            writer.append_snapshot(2, 2_000, &payload2).unwrap();
            writer.sync().unwrap();
        }
        let records = read_all_records(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].commit_seq, 1);
        assert_eq!(records[1].commit_seq, 2);
        let table = records[0].db.get_table(2).unwrap();
        assert_eq!(table.row_count(), 1);
        let table = records[1].db.get_table(2).unwrap();
        assert_eq!(table.row_count(), 2);
    }

    #[test]
    fn torn_trailing_record_is_truncated_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hist.db-fshistory");
        let db = make_test_db(&[(2, 1, "alpha")]);
        {
            let mut writer = HistoryWriter::open(&path).unwrap();
            let payload = snapshot_payload_from_memdb(&db, &[2]);
            writer.append_snapshot(1, 1_000, &payload).unwrap();
            writer.sync().unwrap();
        }
        // Truncate the last byte of the file to simulate a torn write.
        let len = std::fs::metadata(&path).unwrap().len();
        let trimmed = OpenOptions::new().write(true).open(&path).unwrap();
        trimmed.set_len(len - 1).unwrap();
        // Reopen — should silently truncate the torn record back to header.
        {
            let _writer = HistoryWriter::open(&path).unwrap();
        }
        let records = read_all_records(&path).unwrap();
        assert_eq!(records.len(), 0, "torn record should be dropped on reopen");
    }

    #[test]
    fn checksum_mismatch_truncates_safely() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hist.db-fshistory");
        let db = make_test_db(&[(2, 1, "alpha")]);
        {
            let mut writer = HistoryWriter::open(&path).unwrap();
            let payload = snapshot_payload_from_memdb(&db, &[2]);
            writer.append_snapshot(1, 1_000, &payload).unwrap();
            writer.sync().unwrap();
        }
        // Flip the last payload byte to corrupt the checksum.
        {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            let len = file.metadata().unwrap().len();
            file.seek(SeekFrom::Start(len - 1)).unwrap();
            let mut byte = [0u8; 1];
            file.read_exact(&mut byte).unwrap();
            byte[0] ^= 0xFF;
            file.seek(SeekFrom::Start(len - 1)).unwrap();
            file.write_all(&byte).unwrap();
        }
        let records = read_all_records(&path).unwrap();
        assert_eq!(records.len(), 0, "corrupt record should be skipped");
    }

    #[test]
    fn compaction_keeps_only_last_n() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hist.db-fshistory");
        let mut writer = HistoryWriter::open(&path).unwrap();
        for seq in 1..=10u64 {
            let db = make_test_db(&[(2, seq as i64, "alpha")]);
            let payload = snapshot_payload_from_memdb(&db, &[2]);
            writer.append_snapshot(seq, seq * 100, &payload).unwrap();
        }
        writer.sync().unwrap();
        let kept = writer.compact(3).unwrap();
        assert_eq!(kept, 3);
        let records = read_all_records(&path).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].commit_seq, 8);
        assert_eq!(records[1].commit_seq, 9);
        assert_eq!(records[2].commit_seq, 10);
    }

    #[test]
    fn header_validation_rejects_bad_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hist.db-fshistory");
        std::fs::write(&path, b"NOTAFRANKENHISTORY").unwrap();
        let result = HistoryWriter::open(&path);
        assert!(result.is_err(), "bad magic should be rejected");
    }
}
