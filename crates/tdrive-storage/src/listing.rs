//! Detached, read-only directory listings compatible with Go projection/read.go.
//!
//! No Telegram, mutable store, writer lock or daemon API exists in this module.
//! Only invoke the public entrypoint on a detached Go-schema snapshot.

use crate::{SnapshotError, inspect_connection, open_verified_snapshot};
use rusqlite::{Connection, params};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderSlim {
    pub id: String,
    pub name: String,
    pub parent_id: String,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSlim {
    pub msg_id: i64,
    pub content_msg_id: i64,
    pub content_hash: String,
    pub revision: i64,
    pub upload_uuid: String,
    pub part_count: i64,
    pub name: String,
    pub size: i64,
    pub parent_id: String,
    pub upload_time: i64,
    pub uploader_id: i64,
    pub encrypted: bool,
    pub plaintext_size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderListing {
    pub folders: Vec<FolderSlim>,
    pub files: Vec<FileSlim>,
}

/// Lists only the live, immediate descendants in one channel.
///
/// A detached standalone SQLite backup is mandatory. Every call validates
/// Go schema v16 and SQLite integrity; it never creates or writes a database.
/// Root is the empty parent ID, not "/" or "root".
pub fn list_folder_contents_snapshot(
    path: &Path,
    channel_id: i64,
    parent_id: &str,
) -> Result<FolderListing, SnapshotError> {
    let connection = open_verified_snapshot(path)?;
    inspect_connection(&connection)?;
    list_folder_contents_connection(&connection, channel_id, parent_id)
}

fn list_folder_contents_connection(
    conn: &Connection,
    channel_id: i64,
    parent_id: &str,
) -> Result<FolderListing, SnapshotError> {
    // These two SELECT statements intentionally mirror the Go oracle, including
    // its distinct folder and file sorting rules and soft-delete behavior.
    let mut folder_stmt = conn.prepare(
        "SELECT f.id, d.display_name, f.parent_id, d.revision
         FROM dirents d
         JOIN folders f ON f.channel_id = d.channel_id AND f.id = d.object_id
         WHERE d.channel_id = ?1 AND d.parent_id = ?2 AND d.object_kind = 'folder'
           AND d.tombstoned = 0 AND f.tombstoned = 0
         ORDER BY d.display_name, d.object_id",
    )?;
    let folder_rows = folder_stmt.query_map(params![channel_id, parent_id], |row| {
        Ok(FolderSlim {
            id: row.get(0)?,
            name: row.get(1)?,
            parent_id: row.get(2)?,
            revision: row.get(3)?,
        })
    })?;
    let mut folders = Vec::new();
    for row in folder_rows {
        folders.push(row?);
    }

    let mut file_stmt = conn.prepare(
        "SELECT f.msg_id, f.content_msg_id, f.content_hash, f.revision,
                f.upload_uuid, f.part_count, d.display_name, f.size, f.parent_id,
                f.upload_time, f.uploader_user_id, f.encrypted, f.plaintext_size
         FROM files f
         JOIN dirents d
           ON d.channel_id = f.channel_id AND d.object_id = 'f:' || f.msg_id
         WHERE f.channel_id = ?1 AND f.parent_id = ?2 AND f.tombstoned = 0
           AND d.tombstoned = 0
         ORDER BY upload_time DESC, msg_id DESC",
    )?;
    let file_rows = file_stmt.query_map(params![channel_id, parent_id], |row| {
        Ok(FileSlim {
            msg_id: row.get(0)?,
            content_msg_id: row.get(1)?,
            content_hash: row.get(2)?,
            revision: row.get(3)?,
            upload_uuid: row.get(4)?,
            part_count: row.get(5)?,
            name: row.get(6)?,
            size: row.get(7)?,
            parent_id: row.get(8)?,
            upload_time: row.get(9)?,
            uploader_id: row.get(10)?,
            encrypted: row.get::<_, i64>(11)? == 1,
            plaintext_size: row.get(12)?,
        })
    })?;
    let mut files = Vec::new();
    for row in file_rows {
        files.push(row?);
    }
    Ok(FolderListing { folders, files })
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn bounded_listing_rejects_excess_rows_or_text_without_truncation() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE folders(channel_id INTEGER,id TEXT,parent_id TEXT,tombstoned INTEGER);
             CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,parent_id TEXT,object_kind TEXT,display_name TEXT,revision INTEGER,tombstoned INTEGER);
             CREATE TABLE files(channel_id INTEGER,msg_id INTEGER,content_msg_id INTEGER,content_hash TEXT,revision INTEGER,upload_uuid TEXT,part_count INTEGER,size INTEGER,parent_id TEXT,upload_time INTEGER,uploader_user_id INTEGER,encrypted INTEGER,plaintext_size INTEGER,tombstoned INTEGER);
             INSERT INTO folders VALUES(1,'d:a','',0),(1,'d:b','',0);
             INSERT INTO dirents VALUES(1,'d:a','','folder','alpha',1,0),(1,'d:b','','folder','beta',1,0);",
        ).unwrap();
        assert!(matches!(
            list_folder_contents_bounded_connection(&db, 1, "", 1, 100),
            Err(SnapshotError::SnapshotTooLarge)
        ));
        assert!(matches!(
            list_folder_contents_bounded_connection(&db, 1, "", 3, 5),
            Err(SnapshotError::SnapshotTooLarge)
        ));
        let listing = list_folder_contents_bounded_connection(&db, 1, "", 2, 100).unwrap();
        assert_eq!(listing.folders.len(), 2);
        assert!(listing.files.is_empty());
    }
}
