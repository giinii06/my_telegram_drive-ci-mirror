#![cfg(unix)]
use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_mtproto::inbox::{CatchupWindow, CatchupDecision, CatchupError, Inbox, InboxError, InboxEvent, InboxOutcome};

static NEXT: AtomicU64 = AtomicU64::new(0);
const FIRST: &str = "TDX1|t=mkdir|obj=d:docs|p=|n=Docs";
const CHANGED: &str = "TDX1|t=mkdir|obj=d:docs|p=|n=Other";

fn event<'a>(header: &'a str) -> InboxEvent<'a> {
    InboxEvent { peer_key: "-1001234567890", msg_id: 42, raw_header: header }
}
struct PrivateDir(PathBuf);
impl PrivateDir {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tdrive-inbox-{}-{stamp}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&dir).unwrap();
        Self(dir)
    }
    fn db(&self) -> PathBuf { self.0.join("private.sqlite") }
}
impl Drop for PrivateDir {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

#[test]
fn insert_redelivery_and_tamper_keep_first_seen_canonical() {
    let mut inbox = Inbox::memory().unwrap();
    assert_eq!(inbox.append(&event(FIRST)).unwrap(), InboxOutcome::Inserted);
    assert_eq!(inbox.append(&event(FIRST)).unwrap(), InboxOutcome::AlreadySeen);
    assert_eq!(inbox.append(&event(CHANGED)).unwrap(), InboxOutcome::TamperRecorded);
    assert_eq!(inbox.append(&event(CHANGED)).unwrap(), InboxOutcome::TamperRecorded);
    let stats = inbox.stats().unwrap();
    assert_eq!(stats.messages, 1);
    assert_eq!(stats.tampered, 1);
    assert_eq!(inbox.original_header("-1001234567890", 42).unwrap(), Some(FIRST.into()));
}

#[test]
fn malformed_tdx1_ids_and_bounds_reject_without_mutation() {
    let mut inbox = Inbox::memory().unwrap();
    for (peer, id, value) in [
        ("", 42, FIRST),
        ("peer/name", 42, FIRST),
        ("-1001234567890", 0, FIRST),
        ("-1001234567890", 42, "TDX1|t=unknown"),
        ("-1001234567890", 42, "hello"),
    ] {
        let e = InboxEvent { peer_key: peer, msg_id: id, raw_header: value };
        assert!(inbox.append(&e).is_err(), "unexpected accepted inbox event");
    }
    let oversized = "x".repeat(16_385);
    assert!(inbox.append(&event(&oversized)).is_err());
    assert_eq!(inbox.stats().unwrap().messages, 0);
}

#[test]
fn private_sqlite_inbox_survives_restarts_without_exposing_files() {
    let dir = PrivateDir::new();
    let db = dir.db();
    {
        let mut inbox = Inbox::open(&db).unwrap();
        assert_eq!(inbox.append(&event(FIRST)).unwrap(), InboxOutcome::Inserted);
    }
    assert_eq!(fs::metadata(&db).unwrap().permissions().mode() & 0o077, 0);
    {
        let mut inbox = Inbox::open(&db).unwrap();
        assert_eq!(inbox.append(&event(FIRST)).unwrap(), InboxOutcome::AlreadySeen);
        assert_eq!(inbox.stats().unwrap().messages, 1);
    }
    assert!(!dir.0.join("go-v16.sqlite").exists());
}

#[test]
fn wrong_schema_and_untrusted_symlinks_fail_closed() {
    let dir = PrivateDir::new();
    let db = dir.db();
    drop(Inbox::open(&db).unwrap());
    {
        let sqlite = rusqlite::Connection::open(&db).unwrap();
        sqlite.pragma_update(None, "user_version", 2).unwrap();
    }
    assert!(matches!(Inbox::open(&db), Err(InboxError::WrongSchema)));

    let link = dir.0.join("linked.sqlite");
    std::os::unix::fs::symlink(&db, &link).unwrap();
    assert!(matches!(Inbox::open(&link), Err(InboxError::UnsafePath)));
}

#[test]
fn offline_stats_never_creates_missing_database() {
    let dir = PrivateDir::new();
    let db = dir.db();
    assert!(matches!(Inbox::open_existing(&db), Err(InboxError::UnsafePath)));
    assert!(!db.exists());
    {
        let mut writer = Inbox::open(&db).unwrap();
        writer.append(&event(FIRST)).unwrap();
    }
    let readonly = Inbox::open_existing(&db).unwrap();
    assert_eq!(readonly.stats().unwrap().messages, 1);
    assert_eq!(readonly.stats().unwrap().tampered, 0);
}

#[test]
fn completed_sync_cursor_persists_monotonic_across_restart() {
    let dir = PrivateDir::new();
    let db = dir.db();
    let peer = "channel:1234567890";
    {
        let mut inbox = Inbox::open(&db).unwrap();
        assert_eq!(inbox.high_watermark(peer).unwrap(), 0);
        inbox.mark_scanned(peer, 0, 50).unwrap();
        assert_eq!(inbox.high_watermark(peer).unwrap(), 50);
    }
    {
        let mut inbox = Inbox::open(&db).unwrap();
        assert_eq!(inbox.high_watermark(peer).unwrap(), 50);
        assert!(inbox.mark_scanned(peer, 50, 100).is_ok());
        assert_eq!(inbox.high_watermark(peer).unwrap(), 100);
        assert_eq!(inbox.high_watermark("channel:other").unwrap(), 0);
    }
    assert_eq!(Inbox::open_existing(&db).unwrap().high_watermark(peer).unwrap(), 100);
}

#[test]
fn stale_or_regressing_cursor_fails_without_changing_checkpoint() {
    let mut inbox = Inbox::memory().unwrap();
    let peer = "channel:123";
    inbox.mark_scanned(peer, 0, 30).unwrap();
    assert!(matches!(
        inbox.mark_scanned(peer, 0, 50),
        Err(InboxError::StaleCursor)
    ));
    assert!(matches!(
        inbox.mark_scanned(peer, 30, 29),
        Err(InboxError::InvalidEntry)
    ));
    assert_eq!(inbox.high_watermark(peer).unwrap(), 30);
    assert!(matches!(
        inbox.mark_scanned("../escape", 0, 40),
        Err(InboxError::InvalidEntry)
    ));
}

#[test]
fn failed_or_partial_ingest_does_not_checkpoint_prior_inbox_writes() {
    let mut inbox = Inbox::memory().unwrap();
    let peer = "channel:123";
    assert_eq!(inbox.append(&InboxEvent {
        peer_key: peer, msg_id: 80, raw_header: FIRST
    }).unwrap(), InboxOutcome::Inserted);
    // A simulated transport failure or exhausted budget prevents mark_scanned:
    // already stored messages stay idempotent and the cursor stays at zero.
    assert_eq!(inbox.high_watermark(peer).unwrap(), 0);
    assert_eq!(inbox.append(&InboxEvent {
        peer_key: peer, msg_id: 80, raw_header: FIRST
    }).unwrap(), InboxOutcome::AlreadySeen);
    inbox.mark_scanned(peer, 0, 80).unwrap();
    assert_eq!(inbox.high_watermark(peer).unwrap(), 80);
}

#[test]
fn legacy_native_v1_inbox_adds_cursor_without_losing_headers() {
    let dir = PrivateDir::new();
    let db = dir.db();
    {
        let mut inbox = Inbox::open(&db).unwrap();
        inbox.append(&event(FIRST)).unwrap();
    }
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("DROP TABLE native_inbox_progress;").unwrap();
    }
    let mut reopened = Inbox::open(&db).unwrap();
    assert_eq!(reopened.high_watermark("-1001234567890").unwrap(), 0);
    reopened.mark_scanned("-1001234567890", 0, 42).unwrap();
    assert_eq!(reopened.original_header("-1001234567890",42).unwrap(),Some(FIRST.into()));
}

#[test]
fn catchup_window_exhaustion_proves_newest_message_only_after_complete_scan() {
    let mut window = CatchupWindow::new(50, 3).unwrap();
    assert_eq!(window.observe(53).unwrap(), CatchupDecision::Append);
    assert_eq!(window.observe(52).unwrap(), CatchupDecision::Append);
    assert_eq!(window.observe(51).unwrap(), CatchupDecision::Append);
    assert_eq!(window.observe(50).unwrap(), CatchupDecision::Complete);
    assert_eq!(window.completed_newest(), Some(53));
    assert_eq!(window.processed(), 3);
}

#[test]
fn catchup_window_refuses_budget_truncation_and_out_of_order_messages() {
    let mut window = CatchupWindow::new(20, 2).unwrap();
    assert_eq!(window.observe(25).unwrap(), CatchupDecision::Append);
    assert_eq!(window.observe(24).unwrap(), CatchupDecision::Append);
    assert_eq!(window.observe(23), Err(CatchupError::BudgetExhausted));
    assert_eq!(window.completed_newest(), None);
    let mut window = CatchupWindow::new(0, 5).unwrap();
    window.observe(100).unwrap();
    assert_eq!(window.observe(100), Err(CatchupError::OutOfOrder));
    assert_eq!(window.completed_newest(), None);
}

#[test]
fn empty_history_and_unknown_first_cursor_are_not_accidental_success() {
    let mut empty = CatchupWindow::new(0, 10).unwrap();
    empty.exhaust();
    assert_eq!(empty.completed_newest(), Some(0));
    let mut partial = CatchupWindow::new(0, 2).unwrap();
    partial.observe(101).unwrap();
    partial.observe(100).unwrap();
    assert_eq!(partial.completed_newest(), None);
    partial.exhaust();
    assert_eq!(partial.completed_newest(), Some(101));
}
