//! Detached, read-only directory listings compatible with Go projection/read.go.
//!
//! No Telegram, mutable store, writer lock or daemon API exists in this module.
//! Only invoke the public entrypoint on a detached Go-schema snapshot.

use crate::{SnapshotError, inspect_connection, open_verified_snapshot};
use rusqlite::{Connection, Row, params, types::ValueRef};
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
    list_with_budget(conn, channel_id, parent_id, None)
}

/// Bounded API-oriented listing. Rejects rather than truncating rows, and
/// charges SQLite text lengths before cloning them into Rust-owned strings.
pub fn list_folder_contents_snapshot_bounded(
    path: &Path,
    channel_id: i64,
    parent_id: &str,
    max_rows: usize,
    max_text_bytes: usize,
) -> Result<FolderListing, SnapshotError> {
    let connection = open_verified_snapshot(path)?;
    inspect_connection(&connection)?;
    list_folder_contents_bounded_connection(
        &connection, channel_id, parent_id, max_rows, max_text_bytes,
    )
}

fn list_folder_contents_bounded_connection(
    conn: &Connection,
    channel_id: i64,
    parent_id: &str,
    max_rows: usize,
    max_text_bytes: usize,
) -> Result<FolderListing, SnapshotError> {
    if max_rows == 0 || max_rows > 100_000 || max_text_bytes == 0 {
        return Err(SnapshotError::SnapshotTooLarge);
    }
    list_with_budget(conn, channel_id, parent_id, Some(ListingBudget {
        rows: max_rows,
        bytes: max_text_bytes,
        query_limit: max_rows as i64 + 1,
    }))
}

struct ListingBudget {
    rows: usize,
    bytes: usize,
    query_limit: i64,
}

fn charge(
    budget: &mut Option<ListingBudget>,
    row: &Row<'_>,
    columns: &[usize],
) -> Result<(), SnapshotError> {
    if let Some(budget) = budget {
        budget.rows = budget.rows.checked_sub(1).ok_or(SnapshotError::SnapshotTooLarge)?;
        for &column in columns {
            if let ValueRef::Text(bytes) = row.get_ref(column)? {
                budget.bytes = budget.bytes.checked_sub(bytes.len())
                    .ok_or(SnapshotError::SnapshotTooLarge)?;
            }
        }
    }
    Ok(())
}

fn list_with_budget(
    conn: &Connection,
    channel_id: i64,
    parent_id: &str,
    mut budget: Option<ListingBudget>,
) -> Result<FolderListing, SnapshotError> {
    let query_limit = budget.as_ref().map_or(-1, |budget| budget.query_limit);
    // These two SELECT statements intentionally mirror the Go oracle, including
    // its distinct folder and file sorting rules and soft-delete behavior.
    let mut folder_stmt = conn.prepare(
        "SELECT f.id, d.display_name, f.parent_id, d.revision
         FROM dirents d
         JOIN folders f ON f.channel_id = d.channel_id AND f.id = d.object_id
         WHERE d.channel_id = ?1 AND d.parent_id = ?2 AND d.object_kind = 'folder'
           AND d.tombstoned = 0 AND f.tombstoned = 0
         ORDER BY d.display_name, d.object_id LIMIT ?3",
    )?;
    let mut folder_rows = folder_stmt.query(params![channel_id, parent_id, query_limit])?;
    let mut folders = Vec::new();
    while let Some(row) = folder_rows.next()? {
        charge(&mut budget, row, &[0, 1, 2])?;
        folders.push(FolderSlim {
            id: row.get(0)?,
            name: row.get(1)?,
            parent_id: row.get(2)?,
            revision: row.get(3)?,
        });
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
         ORDER BY upload_time DESC, msg_id DESC LIMIT ?3",
    )?;
    let mut file_rows = file_stmt.query(params![channel_id, parent_id, query_limit])?;
    let mut files = Vec::new();
    while let Some(row) = file_rows.next()? {
        charge(&mut budget, row, &[2, 4, 6, 8])?;
        files.push(FileSlim {
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
        });
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


#[derive(Debug)]
pub struct DavEntry {
    pub name: String,
    pub collection: bool,
    pub size: Option<i64>,
}

#[derive(Debug)]
pub struct DavListing {
    pub resource: DavEntry,
    pub children: Vec<DavEntry>,
}

fn dav_file(file: &FileSlim) -> Result<DavEntry, SnapshotError> {
    if file.size < 0 {
        return Err(SnapshotError::InvalidObjectIds);
    }
    Ok(DavEntry {
        name: file.name.clone(),
        collection: false,
        size: if file.encrypted { None } else { Some(file.size) },
    })
}

fn dav_folder(name: String) -> DavEntry {
    DavEntry { name, collection: true, size: None }
}

/// Resolve exact, case-sensitive display-name segments within a detached
/// channel. No filesystem path, fuzzy alias, live mutation or vault unlock.
/// Depth and row/text budgets bound the diagnostic metadata materialization.
pub fn dav_listing_snapshot(
    path: &Path,
    channel_id: i64,
    segments: &[String],
    children: bool,
) -> Result<Option<DavListing>, SnapshotError> {
    use rusqlite::OptionalExtension;

    if channel_id <= 0 || segments.len() > 16 {
        return Err(SnapshotError::InvalidObjectIds);
    }
    let conn = open_verified_snapshot(path)?;
    inspect_connection(&conn)?;
    let exists: Option<i64> = conn.query_row(
        "SELECT 1 FROM channels WHERE channel_id=?1 LIMIT 1", [channel_id], |r| r.get(0),
    ).optional()?;
    if exists.is_none() {
        return Ok(None);
    }
    let mut parent = String::new();
    let mut resource = dav_folder(channel_id.to_string());
    for (index, segment) in segments.iter().enumerate() {
        let listing = list_folder_contents_bounded_connection(&conn, channel_id, &parent, 10_000, 1024 * 1024)?;
        let mut folders = listing.folders.into_iter().filter(|item| &item.name == segment);
        let mut files = listing.files.into_iter().filter(|item| &item.name == segment);
        let folder = folders.next();
        let file = files.next();
        if folders.next().is_some() || files.next().is_some() || (folder.is_some() && file.is_some()) {
            return Err(SnapshotError::InvalidObjectIds);
        }
        match (folder, file) {
            (Some(folder), None) => {
                parent = folder.id;
                resource = dav_folder(folder.name);
            }
            (None, Some(file)) if index + 1 == segments.len() => {
                resource = dav_file(&file)?;
            }
            _ => return Ok(None),
        }
    }
    let mut entries = Vec::new();
    if children && resource.collection {
        let listing = list_folder_contents_bounded_connection(&conn, channel_id, &parent, 10_000, 1024 * 1024)?;
        let mut seen = std::collections::HashSet::new();
        for folder in listing.folders {
            if !seen.insert(folder.name.clone()) { return Err(SnapshotError::InvalidObjectIds); }
            entries.push(dav_folder(folder.name));
        }
        for file in listing.files {
            if !seen.insert(file.name.clone()) { return Err(SnapshotError::InvalidObjectIds); }
            entries.push(dav_file(&file)?);
        }
    }
    Ok(Some(DavListing { resource, children: entries }))
}
