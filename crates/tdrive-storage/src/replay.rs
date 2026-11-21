//! Bounded, immutable replay_log cursor reads for detached Go SQLite v16 files.
//! Go ProjectFromOp calculates first_seen_hash using SHA-256(raw_header).

use crate::{SnapshotError, inspect_connection, open_verified_snapshot};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const MAX_REPLAY_PAGE: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEntry {
    pub channel_id: i64,
    pub msg_id: i64,
    pub op_type: String,
    pub op_payload_json: String,
    pub raw_header: String,
    pub first_seen_hash: String,
    pub actor_user_id: i64,
    pub seen_at: i64,
}

/// Detached immutable copy only: never pass a production, live WAL database.
/// Rows are ordered by msg_id, with after_msg_id exclusive.
pub fn read_replay_page_snapshot(
    path: &Path,
    channel_id: i64,
    after_msg_id: i64,
    limit: usize,
) -> Result<Vec<ReplayEntry>, SnapshotError> {
    if channel_id <= 0 || after_msg_id < 0 || !(1..=MAX_REPLAY_PAGE).contains(&limit) {
        return Err(SnapshotError::InvalidReplayCursor);
    }
    let connection = open_verified_snapshot(path)?;
    inspect_connection(&connection)?;
    read_replay_page_connection(&connection, channel_id, after_msg_id, limit)
}

fn read_replay_page_connection(
    connection: &Connection,
    channel_id: i64,
    after_msg_id: i64,
    limit: usize,
) -> Result<Vec<ReplayEntry>, SnapshotError> {
    let mut stmt = connection.prepare(
        "SELECT channel_id, msg_id, op_type, op_payload_json, raw_header,
                first_seen_hash, actor_user_id, seen_at
         FROM replay_log
         WHERE channel_id = ?1 AND msg_id > ?2
         ORDER BY msg_id ASC LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![channel_id, after_msg_id, limit as i64], |row| {
        Ok(ReplayEntry {
            channel_id: row.get(0)?,
            msg_id: row.get(1)?,
            op_type: row.get(2)?,
            op_payload_json: row.get(3)?,
            raw_header: row.get(4)?,
            first_seen_hash: row.get(5)?,
            actor_user_id: row.get(6)?,
            seen_at: row.get(7)?,
        })
    })?;
    let mut page = Vec::with_capacity(limit);
    for row in rows {
        let entry = row?;
        let actual = format!("{:x}", Sha256::digest(entry.raw_header.as_bytes()));
        if actual != entry.first_seen_hash {
            return Err(SnapshotError::ReplayHashMismatch);
        }
        page.push(entry);
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE replay_log (
                channel_id INTEGER NOT NULL, msg_id INTEGER NOT NULL,
                op_type TEXT NOT NULL, op_payload_json TEXT NOT NULL,
                raw_header TEXT NOT NULL, first_seen_hash TEXT NOT NULL,
                actor_user_id INTEGER NOT NULL, seen_at INTEGER NOT NULL,
                PRIMARY KEY(channel_id, msg_id)
            )",
        ).unwrap();
        connection
    }

    #[test]
    fn rejects_tampered_canonical_header() {
        let connection = database();
        connection.execute(
            "INSERT INTO replay_log VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                123_i64, 7_i64, "mkdir", "{}",
                "TDX1|t=mkdir|obj=d:example|p=|n=Bad",
                "0000000000000000000000000000000000000000000000000000000000000000",
                10_i64, 1700000000_i64,
            ],
        ).unwrap();
        assert!(matches!(
            read_replay_page_connection(&connection, 123, 0, 10),
            Err(SnapshotError::ReplayHashMismatch)
        ));
    }

    #[test]
    fn verifies_ordered_pages_and_channel_isolation() {
        let connection = database();
        for (channel, message) in [(123_i64, 9_i64), (123, 3), (987, 4)] {
            let raw_header = format!("TDX1|t=tomb|obj=f:{message}");
            let hash = format!("{:x}", Sha256::digest(raw_header.as_bytes()));
            connection.execute(
                "INSERT INTO replay_log VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![channel, message, "tomb", "{}", raw_header, hash, 5_i64, 99_i64],
            ).unwrap();
        }
        let first = read_replay_page_connection(&connection, 123, 0, 1).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].msg_id, 3);
        let second = read_replay_page_connection(&connection, 123, 3, 10).unwrap();
        assert_eq!(second.iter().map(|row| row.msg_id).collect::<Vec<_>>(), vec![9]);
        assert!(read_replay_page_connection(&connection, 123, 9, 10).unwrap().is_empty());
    }
}
