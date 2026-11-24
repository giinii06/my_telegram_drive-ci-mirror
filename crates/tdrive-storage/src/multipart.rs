//! Detached snapshot multipart manifest validation and lookup.
use crate::{SnapshotError, inspect_connection, open_verified_snapshot};
use rusqlite::{Connection, OptionalExtension, params};
use std::{error::Error, fmt, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePart {
    pub part_index: i64,
    pub msg_id: i64,
    pub size: i64,
}

#[derive(Debug)]
pub enum MultipartError {
    InvalidManifest,
    InvalidPart,
    Incomplete,
    Snapshot(SnapshotError),
}

impl fmt::Display for MultipartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidManifest => "invalid multipart manifest",
            Self::InvalidPart => "invalid multipart part",
            Self::Incomplete => "incomplete multipart file",
            Self::Snapshot(_) => "multipart snapshot access failed",
        })
    }
}
impl Error for MultipartError {}
impl From<SnapshotError> for MultipartError {
    fn from(error: SnapshotError) -> Self {
        Self::Snapshot(error)
    }
}
impl From<rusqlite::Error> for MultipartError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Snapshot(SnapshotError::Database(error))
    }
}

/// Verify the same ordered stored-byte count invariant as Go MultipartComplete.
pub fn validate_multipart_parts(
    count: i64,
    size: i64,
    parts: &[FilePart],
) -> Result<(), MultipartError> {
    if count < 0 || size < 0 {
        return Err(MultipartError::InvalidManifest);
    }
    if usize::try_from(count).ok() != Some(parts.len()) {
        return Err(MultipartError::Incomplete);
    }
    let mut remaining = size;
    for (index, part) in parts.iter().enumerate() {
        if i64::try_from(index).ok() != Some(part.part_index) {
            return Err(MultipartError::Incomplete);
        }
        if part.msg_id <= 0 || part.size < 0 || part.size > remaining {
            return Err(MultipartError::InvalidPart);
        }
        remaining -= part.size;
    }
    if remaining != 0 {
        return Err(MultipartError::Incomplete);
    }
    Ok(())
}

/// Missing or single-message files return None. A multipart file returns parts
/// only if complete. The caller owns authorization and revision selection;
/// this detached snapshot lookup never reads Telegram or writes the database.
pub fn resolve_multipart_snapshot(
    path: &Path,
    channel_id: i64,
    file_msg_id: i64,
) -> Result<Option<Vec<FilePart>>, MultipartError> {
    let connection = open_verified_snapshot(path)?;
    inspect_connection(&connection)?;
    resolve_connection(&connection, channel_id, file_msg_id)
}

fn resolve_connection(
    connection: &Connection,
    channel_id: i64,
    file_msg_id: i64,
) -> Result<Option<Vec<FilePart>>, MultipartError> {
    if channel_id <= 0 || file_msg_id <= 0 {
        return Err(MultipartError::InvalidManifest);
    }
    let manifest: Option<(String, i64, i64)> = connection
        .query_row(
            "SELECT upload_uuid, part_count, size FROM files WHERE channel_id=?1 AND msg_id=?2 AND tombstoned=0",
            params![channel_id, file_msg_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((uuid, count, size)) = manifest else {
        return Ok(None);
    };
    if uuid.is_empty() {
        return Ok(None);
    }
    if count <= 0 || count > 100_000 || size < 0 {
        return Err(MultipartError::InvalidManifest);
    }
    let mut statement = connection.prepare(
        "SELECT part_index, msg_id, size FROM file_parts
         WHERE channel_id=?1 AND upload_uuid=?2 ORDER BY part_index",
    )?;
    let rows = statement.query_map(params![channel_id, uuid], |row| {
        Ok(FilePart {
            part_index: row.get(0)?,
            msg_id: row.get(1)?,
            size: row.get(2)?,
        })
    })?;
    let mut parts = Vec::new();
    for row in rows {
        if i64::try_from(parts.len()).ok() == Some(count) {
            return Err(MultipartError::Incomplete);
        }
        parts.push(row?);
    }
    validate_multipart_parts(count, size, &parts)?;
    Ok(Some(parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_channel_scoped_and_refuses_missing_parts() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE files(channel_id INTEGER,msg_id INTEGER,upload_uuid TEXT,part_count INTEGER,size INTEGER,tombstoned INTEGER DEFAULT 0);
             CREATE TABLE file_parts(channel_id INTEGER,upload_uuid TEXT,part_index INTEGER,msg_id INTEGER,size INTEGER);
             INSERT INTO files(channel_id,msg_id,upload_uuid,part_count,size) VALUES(1,9,'upload',2,7),(2,9,'upload',2,7),(1,10,'',0,2);
             INSERT INTO file_parts VALUES(1,'upload',1,4,4),(1,'upload',0,3,3),(2,'upload',0,8,7);",
        )
        .unwrap();
        let parts = resolve_connection(&db, 1, 9).unwrap().unwrap();
        assert_eq!(parts.iter().map(|part| part.msg_id).collect::<Vec<_>>(), [3, 4]);
        assert!(resolve_connection(&db, 1, 10).unwrap().is_none());
        assert!(resolve_connection(&db, 1, 999).unwrap().is_none());
        assert!(matches!(
            resolve_connection(&db, 2, 9),
            Err(MultipartError::Incomplete)
        ));
    }
}
