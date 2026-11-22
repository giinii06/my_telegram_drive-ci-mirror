#![cfg(unix)]
use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_mtproto::inbox::{Inbox, InboxError, InboxEvent, InboxOutcome};

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
