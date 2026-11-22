//! Private, standalone SQLite inbox of signed-in Telegram TDX1 headers.
//!
//! This is NOT Go v16 replay_log, file/folder projection, or a full sync engine.
//! Callers own the private database; only validated headers are accepted.
use crate::policy::validate_session_path;
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use tdrive_projection::parse_wire;

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_PEER_BYTES: usize = 128;
const INBOX_VERSION: i64 = 1;

fn valid_peer_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PEER_BYTES
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b':')
}

#[derive(Debug)]
pub enum InboxError {
    UnsafePath,
    InvalidEntry,
    InvalidWire,
    WrongSchema,
    StaleCursor,
    Database(rusqlite::Error),
    Io(std::io::Error),
}

impl fmt::Display for InboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsafePath => "unsafe private Telegram inbox path",
            Self::InvalidEntry => "invalid Telegram inbox event",
            Self::InvalidWire => "invalid TDX1 operation header",
            Self::WrongSchema => "unsupported native inbox schema version",
            Self::StaleCursor => "native inbox catch-up cursor was changed by another writer",
            Self::Database(_) => "native inbox SQLite error",
            Self::Io(_) => "native inbox I/O error",
        })
    }
}
impl std::error::Error for InboxError {}
impl From<rusqlite::Error> for InboxError {
    fn from(value: rusqlite::Error) -> Self { Self::Database(value) }
}
impl From<std::io::Error> for InboxError {
    fn from(value: std::io::Error) -> Self { Self::Io(value) }
}

pub struct InboxEvent<'a> {
    pub peer_key: &'a str,
    pub msg_id: i64,
    pub raw_header: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxOutcome {
    Inserted,
    AlreadySeen,
    TamperRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InboxStats {
    pub messages: i64,
    pub tampered: i64,
}

pub struct Inbox {
    conn: Connection,
}

impl Inbox {
    pub fn memory() -> Result<Self, InboxError> {
        let mut inbox = Self { conn: Connection::open_in_memory()? };
        inbox.initialize(true)?;
        Ok(inbox)
    }

    /// Refuses non-private and symlinked paths. Creates only a dedicated inbox
    /// in a preexisting private (0700) directory, NEVER the Go v16 database.
    pub fn open(path: &Path) -> Result<Self, InboxError> {
        validate_session_path(path).map_err(|_| InboxError::UnsafePath)?;
        let new_file = match fs::symlink_metadata(path) {
            Ok(_) => false,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => return Err(InboxError::UnsafePath),
        };
        if new_file {
            let _file = OpenOptions::new()
                .write(true).create_new(true).mode(0o600).open(path)?;
        }
        validate_session_path(path).map_err(|_| InboxError::UnsafePath)?;
        let mut inbox = Self { conn: Connection::open(path)? };
        inbox.initialize(new_file)?;
        inbox.ensure_progress_table()?;
        Ok(inbox)
    }

    /// Read-only view for statistics: never creates an absent inbox or writes
    /// a WAL/schema version. Required by the offline inbox-stats command.
    pub fn open_existing(path: &Path) -> Result<Self, InboxError> {
        validate_session_path(path).map_err(|_| InboxError::UnsafePath)?;
        let metadata = fs::symlink_metadata(path).map_err(|_| InboxError::UnsafePath)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(InboxError::UnsafePath);
        }
        let mut inbox = Self {
            conn: Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?,
        };
        inbox.initialize(false)?;
        Ok(inbox)
    }

    fn initialize(&mut self, create: bool) -> Result<(), InboxError> {
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if create && version == 0 {
            let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(
                "CREATE TABLE native_inbox_events (
                    peer_key TEXT NOT NULL,
                    msg_id INTEGER NOT NULL,
                    op_type TEXT NOT NULL,
                    raw_header TEXT NOT NULL,
                    first_hash TEXT NOT NULL,
                    PRIMARY KEY(peer_key,msg_id)
                 );
                 CREATE TABLE native_inbox_tamper (
                    peer_key TEXT NOT NULL,
                    msg_id INTEGER NOT NULL,
                    old_hash TEXT NOT NULL,
                    latest_hash TEXT NOT NULL,
                    observations INTEGER NOT NULL,
                    PRIMARY KEY(peer_key,msg_id),
                    FOREIGN KEY(peer_key,msg_id)
                      REFERENCES native_inbox_events(peer_key,msg_id)
                 );
                 CREATE TABLE native_inbox_progress (
                    peer_key TEXT PRIMARY KEY NOT NULL,
                    max_seen_msg_id INTEGER NOT NULL CHECK(max_seen_msg_id > 0)
                 );
                 PRAGMA user_version = 1;",
            )?;
            tx.commit()?;
        } else if version != INBOX_VERSION {
            return Err(InboxError::WrongSchema);
        }
        // Reject a DB with the version pragma but missing the expected tables.
        self.conn.query_row(
            "SELECT (SELECT count(*) FROM native_inbox_events)
                  + (SELECT count(*) FROM native_inbox_tamper)",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(())
    }

    /// Upgrade an earlier native (non-Go) v1 inbox without discarding its
    /// canonical first-seen messages. Read-only opens never run this DDL.
    fn ensure_progress_table(&mut self) -> Result<(), InboxError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS native_inbox_progress (
                 peer_key TEXT PRIMARY KEY NOT NULL,
                 max_seen_msg_id INTEGER NOT NULL CHECK(max_seen_msg_id > 0)
             )"
        )?;
        Ok(())
    }

    /// Highest *fully scanned* Telegram message ID, never merely the highest
    /// locally observed event. It is zero on first run.
    pub fn high_watermark(&self, peer_key: &str) -> Result<i64, InboxError> {
        if !valid_peer_key(peer_key) {
            return Err(InboxError::InvalidEntry);
        }
        let current: Option<i64> = self.conn.query_row(
            "SELECT max_seen_msg_id FROM native_inbox_progress WHERE peer_key=?1",
            params![peer_key],
            |row| row.get(0),
        ).optional()?;
        Ok(current.unwrap_or(0))
    }

    /// Commit a high-watermark ONLY after the caller has scanned the entire
    /// interval (previous, newest]. A transport error / budget exhaustion
    /// MUST skip this call. IMMEDIATE transaction + CAS prevents regressions.
    pub fn mark_scanned(
        &mut self,
        peer_key: &str,
        previous: i64,
        newest: i64,
    ) -> Result<(), InboxError> {
        if !valid_peer_key(peer_key) || previous < 0 || newest < previous {
            return Err(InboxError::InvalidEntry);
        }
        let transaction = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actual: Option<i64> = transaction.query_row(
            "SELECT max_seen_msg_id FROM native_inbox_progress WHERE peer_key=?1",
            params![peer_key],
            |row| row.get(0),
        ).optional()?;
        if actual.unwrap_or(0) != previous {
            return Err(InboxError::StaleCursor);
        }
        if newest > previous {
            transaction.execute(
                "INSERT INTO native_inbox_progress(peer_key,max_seen_msg_id) VALUES(?1,?2)
                 ON CONFLICT(peer_key) DO UPDATE SET max_seen_msg_id=excluded.max_seen_msg_id",
                params![peer_key, newest],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// First seen event is canonical; edits create audit-only tamper entries.
    /// This is a metadata inbox and must not be used as an applied projection.
    pub fn append(&mut self, event: &InboxEvent<'_>) -> Result<InboxOutcome, InboxError> {
        if !valid_peer_key(event.peer_key)
            || event.msg_id <= 0
            || !event.raw_header.starts_with("TDX1|")
            || event.raw_header.len() > MAX_HEADER_BYTES
            || event.raw_header.contains('\n')
            || event.raw_header.contains('\0')
        {
            return Err(InboxError::InvalidEntry);
        }
        let parsed = parse_wire(event.raw_header).map_err(|_| InboxError::InvalidWire)?;
        let hash = format!("{:x}", Sha256::digest(event.raw_header.as_bytes()));
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT first_hash FROM native_inbox_events WHERE peer_key=?1 AND msg_id=?2",
                params![event.peer_key, event.msg_id],
                |row| row.get(0),
            ).optional()?;
        let outcome = match existing {
            Some(previous) if previous == hash => InboxOutcome::AlreadySeen,
            Some(previous) => {
                tx.execute(
                    "INSERT INTO native_inbox_tamper
                        (peer_key,msg_id,old_hash,latest_hash,observations)
                     VALUES(?1,?2,?3,?4,1)
                     ON CONFLICT(peer_key,msg_id) DO UPDATE SET
                        latest_hash=excluded.latest_hash,
                        observations=native_inbox_tamper.observations+1",
                    params![event.peer_key, event.msg_id, previous, hash],
                )?;
                InboxOutcome::TamperRecorded
            }
            None => {
                tx.execute(
                    "INSERT INTO native_inbox_events
                        (peer_key,msg_id,op_type,raw_header,first_hash)
                     VALUES(?1,?2,?3,?4,?5)",
                    params![event.peer_key, event.msg_id, parsed.op_type.as_str(),
                            event.raw_header, hash],
                )?;
                InboxOutcome::Inserted
            }
        };
        tx.commit()?;
        Ok(outcome)
    }

    pub fn stats(&self) -> Result<InboxStats, InboxError> {
        let (messages, tampered) = self.conn.query_row(
            "SELECT (SELECT COUNT(*) FROM native_inbox_events),
                    (SELECT COUNT(*) FROM native_inbox_tamper)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(InboxStats { messages, tampered })
    }

    pub fn original_header(&self, peer_key: &str, msg_id: i64)
        -> Result<Option<String>, InboxError>
    {
        Ok(self.conn.query_row(
            "SELECT raw_header FROM native_inbox_events
             WHERE peer_key=?1 AND msg_id=?2",
            params![peer_key, msg_id],
            |row| row.get(0),
        ).optional()?)
    }
}
