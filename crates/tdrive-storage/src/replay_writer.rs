//! Transactional Go v16 replay journal writer for an explicitly supplied DB.
//!
//! This module applies ordinary multipart parts and a restricted plain folder
//! operation subset; it does not manage
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
            Self::UnsupportedProjection => "operation is outside the supported projection subset",
            Self::OutOfOrder => "unseen replay message is out of order",
            Self::UnappliedReplay => "existing replay row has no matching projected state",
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
    append_replay_event_inner(connection, event, None)
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
    append_replay_event_inner(connection, &event, Some(Projection::Part(uuid, op.part_index, op.file_size)))
}

#[derive(Clone, Copy)]
enum Projection<'a> {
    Part(&'a str, i64, i64),
    Mkdir { id: &'a str, parent: &'a str, name: &'a str },
    Rmdir { id: &'a str },
}

fn portable_ascii_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 240
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || name.ends_with([' ', '.'])
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b" ._-".contains(&b))
    {
        return false;
    }
    let base = name.split('.').next().unwrap_or_default().trim_end().to_ascii_uppercase();
    !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT"))
             && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

/// Apply the ordinary ASCII mkdir/rmdir subset to a caller-owned disposable
/// Go-v16 clone. Legacy aliases and Unicode folding are intentionally not
/// approximated; unsupported or incoherent state fails closed.
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
    let id = op.object.as_deref().ok_or(ReplayWriteError::InvalidWire)?;
    let (projection, payload): (Projection<'_>, String) = match op.op_type {
        OpType::Mkdir => {
            let name = op.name.as_deref().ok_or(ReplayWriteError::InvalidWire)?;
            if !portable_ascii_name(name) {
                return Err(ReplayWriteError::UnsupportedProjection);
            }
            let payload = connection.query_row(
                "SELECT json_object('Type','mkdir','Obj',?1,'Parent',?2,'Name',?3)",
                params![id, op.parent, name], |r| r.get(0),
            )?;
            (Projection::Mkdir { id, parent: &op.parent, name }, payload)
        }
        OpType::Rmdir => (
            Projection::Rmdir { id },
            connection.query_row("SELECT json_object('Type','rmdir','Obj',?1)", [id], |r| r.get(0))?,
        ),
        _ => return Err(ReplayWriteError::UnsupportedProjection),
    };
    let event = IncomingReplayEvent {
        channel_id, msg_id, op_type: op.op_type.as_str(),
        op_payload_json: &payload, raw_header, actor_user_id, seen_at,
    };
    append_replay_event_inner(connection, &event, Some(projection))
}

type FolderState = (String, String, i64, i64);
type DirentState = (String, String, String, String, i64, i64);

fn folder_state(
    tx: &rusqlite::Transaction<'_>, channel: i64, id: &str,
) -> Result<Option<FolderState>, ReplayWriteError> {
    let folder: Option<FolderState> = tx.query_row(
        "SELECT name,parent_id,revision,tombstoned FROM folders WHERE channel_id=?1 AND id=?2",
        params![channel,id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).optional()?;
    let dirent: Option<DirentState> = tx.query_row(
        "SELECT object_kind,display_name,parent_id,name_key,revision,tombstoned
         FROM dirents WHERE channel_id=?1 AND object_id=?2",
        params![channel,id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
    ).optional()?;
    match (&folder, dirent) {
        (None, None) => Ok(None),
        (Some((name, parent, revision, deleted)), Some((kind, display, dir_parent, key, dir_rev, dir_deleted)))
            if portable_ascii_name(name) && *revision > 0 && matches!(*deleted,0|1)
                && kind == "folder" && display == *name && dir_parent == *parent
                && key == name.to_ascii_lowercase() && dir_rev == *revision && dir_deleted == *deleted => Ok(folder),
        _ => Err(ReplayWriteError::UnappliedReplay),
    }
}

fn apply_projection(
    tx: &rusqlite::Transaction<'_>, event: &IncomingReplayEvent<'_>, projection: Projection<'_>,
    already_seen: bool,
) -> Result<(), ReplayWriteError> {
    let channel = event.channel_id;
    match projection {
        Projection::Part(uuid,index,size) => {
            if !already_seen {
                tx.execute(
                    "INSERT INTO file_parts(channel_id,upload_uuid,part_index,msg_id,size)
                     VALUES(?1,?2,?3,?4,?5)
                     ON CONFLICT(channel_id,upload_uuid,part_index) DO NOTHING",
                    params![channel, uuid, index, event.msg_id, size],
                )?;
            }
            require_projected_part(tx, event, uuid, index, size)
        }
        Projection::Mkdir { id,parent,name } => {
            if folder_state(tx,channel,id)?.is_some() {
                return Ok(());
            }
            if already_seen { return Err(ReplayWriteError::UnappliedReplay); }
            tx.execute("INSERT INTO folders(channel_id,id,name,parent_id,revision,tombstoned)
                VALUES(?1,?2,?3,?4,1,0)", params![channel,id,name,parent])?;
            tx.execute("INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned)
                VALUES(?1,?2,'folder',?3,?4,?5,1,0)", params![channel,id,parent,name,name.to_ascii_lowercase()])?;
            Ok(())
        }
        Projection::Rmdir { id } => {
            let Some((_,_,revision,deleted)) = folder_state(tx,channel,id)? else {
                return Ok(());
            };
            if deleted == 1 { return Ok(()); }
            if already_seen { return Err(ReplayWriteError::UnappliedReplay); }
            let next = revision.checked_add(1).ok_or(ReplayWriteError::UnappliedReplay)?;
            let a = tx.execute("UPDATE folders SET tombstoned=1,revision=?3 WHERE channel_id=?1 AND id=?2 AND revision=?4 AND tombstoned=0",
                params![channel,id,next,revision])?;
            let b = tx.execute("UPDATE dirents SET tombstoned=1,revision=?3 WHERE channel_id=?1 AND object_id=?2 AND revision=?4 AND tombstoned=0",
                params![channel,id,next,revision])?;
            if a != 1 || b != 1 { return Err(ReplayWriteError::UnappliedReplay); }
            Ok(())
        }
    }
}

fn require_projected_part(
    transaction: &rusqlite::Transaction<'_>,
    event: &IncomingReplayEvent<'_>,
    uuid: &str,
    index: i64,
    size: i64,
) -> Result<(), ReplayWriteError> {
    let projected: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT msg_id,size FROM file_parts
             WHERE channel_id=?1 AND upload_uuid=?2 AND part_index=?3",
            params![event.channel_id, uuid, index],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match projected {
        Some((id, stored_size)) if id > 0 && id < event.msg_id && stored_size >= 0 => Ok(()),
        Some((id, stored_size)) if id == event.msg_id && stored_size == size => Ok(()),
        _ => Err(ReplayWriteError::UnappliedReplay),
    }
}

fn append_replay_event_inner(
    connection: &mut Connection,
    event: &IncomingReplayEvent<'_>,
    projection: Option<Projection<'_>>,
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
            if let Some(projection) = projection {
                apply_projection(&transaction, event, projection, true)?;
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
            if projection.is_some() {
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
            if let Some(projection) = projection {
                apply_projection(&transaction, event, projection, false)?;
            }
            ReplayWriteOutcome::Inserted
        }
    };
    transaction.commit()?;
    Ok(outcome)
}
