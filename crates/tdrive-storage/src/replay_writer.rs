//! Transactional Go v16 replay journal writer for an explicitly supplied DB.
//!
//! This module intentionally does NOT apply changes to folders/files or manage
//! SQLite locks, Telegram connections, migrations, or live production paths.
//! The caller must own the only writer and operate on an isolated test clone
//! until full Go projection parity has been verified.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use tdrive_projection::{OpType, parse_wire};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
const LEGACY_VERSION: i64 = 16;

/// Metadata for one already authenticated/received Telegram history message.
///
/// Never derive Debug for this object: raw_header can disclose file names and
/// op_payload_json may include vault metadata. Do not log either.
pub struct IncomingReplayEvent<'a> {
    pub channel_id: i64,
    pub msg_id: i64,
    pub op_type: &'a str,
    pub op_payload_json: &'a str,
    pub raw_header: &'a str,
    pub actor_user_id: i64,
    pub seen_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayWriteOutcome {
    Inserted,
    AlreadySeen,
    TamperRecorded,
}

#[derive(Debug)]
pub enum ReplayWriteError {
    InvalidEntry,
    InvalidWire,
    InvalidPayload,
    WrongSchema,
    Database(rusqlite::Error),
}

impl fmt::Display for ReplayWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Avoid displaying SQL statement or payload content in user-facing logs.
        f.write_str(match self {
            Self::InvalidEntry => "invalid Telegram replay message metadata",
            Self::InvalidWire => "invalid or mismatched Telegram wire operation",
            Self::InvalidPayload => "invalid replay payload JSON object",
            Self::WrongSchema => "expected pre-existing Go schema v16",
            Self::Database(_) => "SQLite replay journal transaction failed",
        })
    }
}

impl Error for ReplayWriteError {}

impl From<rusqlite::Error> for ReplayWriteError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}

/// Append one operation without replacing the first-seen canonical message.
///
/// This only writes replay_log and, on conflicts, replay_log_tamper. It does
/// **not** perform Go's ApplyOp projection mutation. In particular, it is not
/// safe to use it as the complete sync engine or directly on a live Go DB.
/// Callers must hold the backend's exclusive writer lock and pass a disposable
/// Go-v16 clone until all 17 operation types can be applied atomically.
pub fn append_replay_event(
    connection: &mut Connection,
    event: &IncomingReplayEvent<'_>,
) -> Result<ReplayWriteOutcome, ReplayWriteError> {
    if event.channel_id <= 0
        || event.msg_id <= 0
        || event.actor_user_id < 0
        || event.seen_at <= 0
        || event.raw_header.is_empty()
        || event.raw_header.len() > MAX_HEADER_BYTES
        || event.raw_header.contains('\0')
        || event.raw_header.contains('\n')
        || event.op_payload_json.is_empty()
        || event.op_payload_json.len() > MAX_PAYLOAD_BYTES
    {
        return Err(ReplayWriteError::InvalidEntry);
    }

    let decoded = parse_wire(event.raw_header).map_err(|_| ReplayWriteError::InvalidWire)?;
    if OpType::parse(event.op_type) != Some(decoded.op_type) {
        return Err(ReplayWriteError::InvalidWire);
    }

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: Option<i64> = transaction.query_row(
        "SELECT MAX(version) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    if version != Some(LEGACY_VERSION) {
        return Err(ReplayWriteError::WrongSchema);
    }

    // JSON1 is included in bundled SQLite. No user input enters SQL syntax.
    let payload_matches_wire: i64 = transaction.query_row(
        "SELECT CASE WHEN json_valid(?1)
          THEN COALESCE(json_type(?1) = 'object' AND json_extract(?1, '$.Type') = ?2, 0)
          ELSE 0 END",
        params![event.op_payload_json, event.op_type],
        |row| row.get(0),
    )?;
    if payload_matches_wire != 1 {
        return Err(ReplayWriteError::InvalidPayload);
    }

    let incoming_hash = format!("{:x}", Sha256::digest(event.raw_header.as_bytes()));
    let existing: Option<String> = transaction
        .query_row(
            "SELECT first_seen_hash FROM replay_log WHERE channel_id=?1 AND msg_id=?2",
            params![event.channel_id, event.msg_id],
            |row| row.get(0),
        )
        .optional()?;

    let outcome = match existing {
        Some(hash) if hash == incoming_hash => ReplayWriteOutcome::AlreadySeen,
        Some(hash) => {
            transaction.execute(
                "INSERT INTO replay_log_tamper
                   (channel_id,msg_id,old_hash,new_hash,detected_at)
                 VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(channel_id,msg_id) DO UPDATE SET
                   new_hash=excluded.new_hash,
                   detected_at=excluded.detected_at",
                params![
                    event.channel_id,
                    event.msg_id,
                    hash,
                    incoming_hash,
                    event.seen_at,
                ],
            )?;
            ReplayWriteOutcome::TamperRecorded
        }
        None => {
            transaction.execute(
                "INSERT INTO replay_log
                   (channel_id,msg_id,op_type,op_payload_json,raw_header,
                    first_seen_hash,actor_user_id,seen_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    event.channel_id,
                    event.msg_id,
                    event.op_type,
                    event.op_payload_json,
                    event.raw_header,
                    incoming_hash,
                    event.actor_user_id,
                    event.seen_at,
                ],
            )?;
            ReplayWriteOutcome::Inserted
        }
    };
    transaction.commit()?;
    Ok(outcome)
}
