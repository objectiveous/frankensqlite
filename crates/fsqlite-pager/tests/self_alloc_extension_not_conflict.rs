use std::path::Path;

use fsqlite_error::Result;
use fsqlite_pager::traits::{
    CheckpointPageWriter, CheckpointResult, JournalMode, MvccPager, TransactionHandle,
    TransactionMode, WalBackend,
};
use fsqlite_pager::{CheckpointMode, SimplePager};
use fsqlite_types::cx::Cx;
use fsqlite_types::PageSize;
use fsqlite_vfs::MemoryVfs;

#[derive(Default)]
struct NoopWalBackend;

impl WalBackend for NoopWalBackend {
    fn append_frame(
        &mut self,
        _cx: &Cx,
        _page_number: u32,
        _page_data: &[u8],
        _db_size_if_commit: u32,
    ) -> Result<()> {
        Ok(())
    }

    fn read_page(&mut self, _cx: &Cx, _page_number: u32) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn sync(&mut self, _cx: &Cx) -> Result<()> {
        Ok(())
    }

    fn frame_count(&self) -> usize {
        0
    }

    fn checkpoint(
        &mut self,
        _cx: &Cx,
        mode: CheckpointMode,
        _writer: &mut dyn CheckpointPageWriter,
        _backfilled_frames: u32,
        _oldest_reader_frame: Option<u32>,
    ) -> Result<CheckpointResult> {
        Ok(CheckpointResult {
            total_frames: 0,
            frames_backfilled: 0,
            completed: true,
            wal_was_reset: matches!(mode, CheckpointMode::Restart | CheckpointMode::Truncate),
            requested_mode: mode,
            effective_mode: mode,
        })
    }
}

#[test]
fn self_allocated_eof_page_stays_out_of_conflict_surface() {
    let cx = Cx::new();
    let pager = SimplePager::open_with_cx(
        &cx,
        MemoryVfs::new(),
        Path::new("/self_alloc_extension.db"),
        PageSize::DEFAULT,
    )
    .expect("pager should open");
    pager
        .set_wal_backend(Box::new(NoopWalBackend))
        .expect("no-op WAL backend should install");
    pager
        .set_journal_mode(&cx, JournalMode::Wal)
        .expect("WAL mode should be available");

    let mut txn = pager
        .begin(&cx, TransactionMode::Concurrent)
        .expect("concurrent transaction should begin");
    let page = txn.allocate_page(&cx).expect("allocation should extend EOF");
    assert_eq!(
        page.get(),
        2,
        "fresh database should extend from page 1 to page 2"
    );
    txn.write_page(&cx, page, &[0xA5; 64])
        .expect("newly allocated page should accept writes");

    let pending_commit = txn
        .pending_commit_pages()
        .expect("pending commit surface should be available");
    let pending_conflict = txn
        .pending_conflict_pages()
        .expect("pending conflict surface should be available");

    assert!(
        pending_commit.contains(&page),
        "self-allocated extension page must be written at commit"
    );
    assert!(
        !pending_conflict.contains(&page),
        "self-allocated EOF extension page {page:?} must not be treated as a cross-process conflict page; pending_conflict={pending_conflict:?}"
    );
}
