//! Transactional Go v16 replay journal writer for an explicitly supplied DB.
//!
//! This module applies ordinary multipart part metadata and a restricted plain
//! legacy mkdir subset; it does not manage
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
    UnsupportedProjection,
    OutOfOrder,
    UnappliedReplay,
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
            Self::UnsupportedProjection => "operation is outside the ordinary part projector",
            Self::OutOfOrder => "unseen replay message is out of order",
            Self::UnappliedReplay => "existing replay row has no matching projected part",
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
    append_replay_event_inner(connection, event, None, None)
}

/// Project an ordinary multipart part and its journal row atomically on a
/// disposable Go-v16 clone. No live sync, rendition or manifest handling.
/// The caller owns the only writer; no cursor is advanced here.
pub fn project_plain_part(
    connection: &mut Connection,
    channel_id: i64,
    msg_id: i64,
    raw_header: &str,
    actor_user_id: i64,
    seen_at: i64,
) -> Result<ReplayWriteOutcome, ReplayWriteError> {
    if raw_header.len() > MAX_HEADER_BYTES {
        return Err(ReplayWriteError::InvalidEntry);
    }
    let op = parse_wire(raw_header).map_err(|_| ReplayWriteError::InvalidWire)?;
    if op.op_type != OpType::Part
        || op.encrypted
        || op.encryption_version != 0
        || op.plaintext_size != 0
        || op.upload_time != 0
    {
        return Err(ReplayWriteError::UnsupportedProjection);
    }
    let uuid = op.upload_uuid.as_deref().ok_or(ReplayWriteError::InvalidWire)?;
    if uuid.trim().is_empty() {
        return Err(ReplayWriteError::InvalidWire);
    }
    // Derive the replay payload from the parsed wire, never caller-supplied JSON.
    let payload: String = connection.query_row(
        "SELECT json_object('Type','part','UploadUUID',?1,'PartIndex',?2,'FileSize',?3)",
        params![uuid, op.part_index, op.file_size],
        |row| row.get(0),
    )?;
    let event = IncomingReplayEvent {
        channel_id,
        msg_id,
        op_type: "part",
        op_payload_json: &payload,
        raw_header,
        actor_user_id,
        seen_at,
    };
    append_replay_event_inner(
        connection, &event, Some((uuid, op.part_index, op.file_size)), None,
    )
}

/// Project the collision-free portable ASCII subset of legacy mkdir on a
/// disposable clone. Full Unicode normalization/legacy alias parity is pending.
/// Folder and namespace state must be consistent or the transaction fails.
pub fn project_plain_folder(
    connection: &mut Connection,
    channel_id: i64,
    msg_id: i64,
    raw_header: &str,
    actor_user_id: i64,
    seen_at: i64,
) -> Result<ReplayWriteOutcome, ReplayWriteError> {
    if raw_header.len() > MAX_HEADER_BYTES {
        return Err(ReplayWriteError::InvalidEntry);
    }
    let op = parse_wire(raw_header).map_err(|_| ReplayWriteError::InvalidWire)?;
    if op.op_type != OpType::Mkdir
        || op.encrypted
        || op.encrypted_name.is_some()
        || op.encryption_version != 0
        || op.plaintext_size != 0
    {
        return Err(ReplayWriteError::UnsupportedProjection);
    }
    let id = op.object.as_deref().ok_or(ReplayWriteError::InvalidWire)?;
    let name = op.name.as_deref().ok_or(ReplayWriteError::InvalidWire)?;
    if !plain_folder_id(id)
        || (!op.parent.is_empty() && !plain_folder_id(&op.parent))
        || plain_name_key(name).is_none()
    {
        return Err(ReplayWriteError::UnsupportedProjection);
    }
    let payload: String = connection.query_row(
        "SELECT json_object('Type','mkdir','Obj',?1,'Parent',?2,'Name',?3)",
        params![id, op.parent, name],
        |row| row.get(0),
    )?;
    let event = IncomingReplayEvent {
        channel_id,
        msg_id,
        op_type: "mkdir",
        op_payload_json: &payload,
        raw_header,
        actor_user_id,
        seen_at,
    };
    append_replay_event_inner(connection, &event, None, Some((id, &op.parent, name)))
}

fn plain_folder_id(id: &str) -> bool {
    id.strip_prefix("d:").is_some_and(|value| {
        !value.is_empty() && value.len() <= 254
            && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    })
}

fn plain_name_key(name: &str) -> Option<String> {
    if name.is_empty() || name.len() > 240 || name.trim().is_empty()
        || name.ends_with([' ', '.'])
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'.' | b'_' | b'-'))
    {
        return None;
    }
    let key = name.to_ascii_lowercase();
    let base = key.split('.').next()?;
    if matches!(base, "con" | "prn" | "aux" | "nul" | "clock$")
        || (base.len() == 4 && (base.starts_with("com") || base.starts_with("lpt"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
    {
        return None;
    }
    Some(key)
}

fn folder_consistent(
    tx: &rusqlite::Transaction<'_>,
    channel: i64,
    id: &str,
) -> Result<bool, ReplayWriteError> {
    let row: Option<(String, String, i64, i64)> = tx.query_row(
        "SELECT name,parent_id,revision,tombstoned FROM folders WHERE channel_id=?1 AND id=?2",
        params![channel, id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?;
    let Some((name, parent, revision, tombstoned)) = row else {
        return Ok(false);
    };
    let key = plain_name_key(&name).ok_or(ReplayWriteError::UnsupportedProjection)?;
    let matches: i64 = tx.query_row(
        "SELECT COUNT(*) FROM dirents WHERE channel_id=?1 AND object_id=?2
           AND object_kind='folder' AND parent_id=?3 AND display_name=?4
           AND name_key=?5 AND revision=?6 AND tombstoned=?7",
        params![channel, id, parent, name, key, revision, tombstoned],
        |r| r.get(0),
    )?;
    if matches != 1 {
        return Err(ReplayWriteError::UnappliedReplay);
    }
    Ok(true)
}

fn append_replay_event_inner(
    connection: &mut Connection,
    event: &IncomingReplayEvent<'_>,
    part: Option<(&str, i64, i64)>,
    folder: Option<(&str, &str, &str)>,
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
        Some(hash) if hash == incoming_hash => {
            if let Some((uuid, index, size)) = part {
                let projected: Option<(i64, i64)> = transaction
                    .query_row(
                        "SELECT msg_id,size FROM file_parts
                         WHERE channel_id=?1 AND upload_uuid=?2 AND part_index=?3",
                        params![event.channel_id, uuid, index],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?;
                match projected {
                    Some((id, _)) if id > 0 && id < event.msg_id => {}
                    Some((id, stored_size)) if id == event.msg_id && stored_size == size => {}
                    _ => return Err(ReplayWriteError::UnappliedReplay),
                }
            }
            if let Some((id, _, _)) = folder {
                if !folder_consistent(&transaction, event.channel_id, id)? {
                    return Err(ReplayWriteError::UnappliedReplay);
                }
            }
            ReplayWriteOutcome::AlreadySeen
        }
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
            if part.is_some() || folder.is_some() {
                let last: Option<i64> = transaction.query_row(
                    "SELECT MAX(msg_id) FROM replay_log WHERE channel_id=?1",
                    [event.channel_id],
                    |row| row.get(0),
                )?;
                if last.is_some_and(|id| id >= event.msg_id) {
                    return Err(ReplayWriteError::OutOfOrder);
                }
            }
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
            if let Some((uuid, index, size)) = part {
                transaction.execute(
                    "INSERT INTO file_parts(channel_id,upload_uuid,part_index,msg_id,size)
                     VALUES(?1,?2,?3,?4,?5)
                     ON CONFLICT(channel_id,upload_uuid,part_index) DO NOTHING",
                    params![event.channel_id, uuid, index, event.msg_id, size],
                )?;
            }
            if let Some((id, parent, name)) = folder {
                if !folder_consistent(&transaction, event.channel_id, id)? {
                    let key = plain_name_key(name).ok_or(ReplayWriteError::UnsupportedProjection)?;
                    transaction.execute(
                        "INSERT INTO folders(channel_id,id,name,parent_id,tombstoned,revision)
                         VALUES(?1,?2,?3,?4,0,1)",
                        params![event.channel_id, id, name, parent],
                    )?;
                    transaction.execute(
                        "INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned)
                         VALUES(?1,?2,'folder',?3,?4,?5,1,0)",
                        params![event.channel_id, id, parent, name, key],
                    )?;
                }
            }
            ReplayWriteOutcome::Inserted
        }
    };
    transaction.commit()?;
    Ok(outcome)
}
