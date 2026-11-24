//! Read-only Go v16 snapshot verification and an isolated, caller-owned replay journal writer.
//!
//! Snapshot inspection never modifies a database. The separate replay_writer
//! API accepts only an explicit caller-owned SQLite connection: never a live
//! daemon DB without an exclusive lock, complete projection parity and rollback.

mod backup;
mod digest;
pub use backup::{
    BackupReceipt, create_consistent_snapshot, restore_verified_clone, verify_snapshot,
};
mod listing;
mod multipart;
mod multipart_assembly;
mod upload_preparation;
pub use upload_preparation::{PreparedPart, UploadPart, UploadReceipt, plan_upload_parts, prepare_upload};
pub use multipart_assembly::{assemble_multipart, copy_exact_part};
pub use multipart::{FilePart, MultipartError, resolve_multipart_snapshot, validate_multipart_parts};
mod replay;
mod replay_writer;
pub use digest::{TableDigest, snapshot_table_digests};
pub use listing::{FileSlim, FolderListing, FolderSlim, list_folder_contents_snapshot, list_folder_contents_snapshot_bounded};
pub use replay::{ReplayEntry, read_replay_page_snapshot};
pub use replay_writer::{
    IncomingReplayEvent, ReplayWriteError, ReplayWriteOutcome, append_replay_event, project_plain_part,
};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub const LEGACY_SCHEMA_VERSION: i64 = 16;

const REQUIRED_TABLES: &[&str] = &[
    "schema_version",
    "channels",
    "replay_log",
    "replay_log_tamper",
    "replay_log_rejects",
    "folders",
    "files",
    "dirents",
    "file_revisions",
    "projection_operations",
    "encryption",
    "file_parts",
    "pending_joins",
    "trash_entries",
];

const REQUIRED_COLUMNS: &[(&str, &[&str])] = &[
    (
        "files",
        &[
            "channel_id",
            "msg_id",
            "parent_id",
            "content_msg_id",
            "revision",
            "encrypted",
            "upload_uuid",
        ],
    ),
    (
        "folders",
        &["channel_id", "id", "parent_id", "revision", "tombstoned"],
    ),
    (
        "replay_log",
        &["channel_id", "msg_id", "raw_header", "first_seen_hash"],
    ),
    (
        "dirents",
        &[
            "channel_id",
            "object_id",
            "parent_id",
            "name_key",
            "revision",
            "tombstoned",
        ],
    ),
    (
        "file_revisions",
        &["channel_id", "file_msg_id", "revision", "content_msg_id"],
    ),
];

#[derive(Debug)]
pub enum SnapshotError {
    UnsafePath,
    ActiveWal,
    InvalidHeader,
    UnsupportedSchema(i64),
    MissingTable(&'static str),
    MissingColumn(&'static str, &'static str),
    InvalidIndex,
    IntegrityFailure,
    ForeignKeyFailure,
    InvalidObjectIds,
    InvalidReplayCursor,
    ReplayHashMismatch,
    SnapshotTooLarge,
    BackupOutputExists,
    UnsafeBackupDestination,
    InvalidBackupManifest,
    Database(rusqlite::Error),
    Io(std::io::Error),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafePath => {
                f.write_str("snapshot must be an existing regular non-symlink file")
            }
            Self::ActiveWal => {
                f.write_str("snapshot has a WAL/SHM/rollback journal sidecar: provide a detached SQLite backup")
            }
            Self::InvalidHeader => f.write_str("file is not a SQLite3 database"),
            Self::UnsupportedSchema(v) => {
                write!(f, "expected migrated Go schema v16, found version {v}")
            }
            Self::MissingTable(t) => write!(f, "required Go table absent: {t}"),
            Self::MissingColumn(t, c) => write!(f, "required Go column absent: {t}.{c}"),
            Self::InvalidIndex => f.write_str("required active sibling uniqueness index absent"),
            Self::IntegrityFailure => f.write_str("SQLite integrity_check did not return ok"),
            Self::ForeignKeyFailure => f.write_str("SQLite foreign_key_check returned violations"),
            Self::InvalidObjectIds => f.write_str("Go namespace identifier invariant violated"),
            Self::InvalidReplayCursor => {
                f.write_str("invalid channel ID, replay cursor, or page limit")
            }
            Self::ReplayHashMismatch => f.write_str("Go replay_log raw header SHA-256 mismatch"),
            Self::SnapshotTooLarge => {
                f.write_str("detached SQLite table exceeds bounded digest limits")
            }
            Self::BackupOutputExists => f.write_str("snapshot or manifest already exists"),
            Self::UnsafeBackupDestination => {
                f.write_str("backup target directory must be private and non-symlink")
            }
            Self::InvalidBackupManifest => {
                f.write_str("snapshot manifest or content verification failed")
            }
            Self::Database(e) => write!(f, "SQLite read failed: {e}"),
            Self::Io(e) => write!(f, "snapshot access failed: {e}"),
        }
    }
}

impl Error for SnapshotError {}

impl From<rusqlite::Error> for SnapshotError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}
impl From<std::io::Error> for SnapshotError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotReport {
    pub schema_version: i64,
    /// Only counts, never names, hashes, captions, secrets or key material.
    pub table_counts: Vec<(&'static str, i64)>,
}

impl SnapshotReport {
    pub fn count(&self, name: &str) -> Option<i64> {
        self.table_counts
            .iter()
            .find(|(table, _)| *table == name)
            .map(|(_, count)| *count)
    }
}

/// Deliberately refuses a live WAL database. The operator must first create
/// a self-contained transactionally-consistent copy through the Go daemon or
/// SQLite backup API, **not** copy the live database's main file alone.
pub fn inspect_snapshot(path: &Path) -> Result<SnapshotReport, SnapshotError> {
    let conn = open_verified_snapshot(path)?;
    inspect_connection(&conn)
}

/// Open a detached snapshot only. The caller must additionally inspect schema
/// integrity before reading rows. Never returns an active Go daemon database.
fn open_verified_snapshot(path: &Path) -> Result<Connection, SnapshotError> {
    let meta = fs::symlink_metadata(path).map_err(SnapshotError::Io)?;
    if !meta.file_type().is_file() || meta.file_type().is_symlink() {
        return Err(SnapshotError::UnsafePath);
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut companion = path.as_os_str().to_os_string();
        companion.push(suffix);
        match fs::symlink_metadata(Path::new(&companion)) {
            Ok(_) => return Err(SnapshotError::ActiveWal),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let mut f = File::open(path)?;
    let mut header = [0u8; 16];
    f.read_exact(&mut header).map_err(SnapshotError::Io)?;
    if &header != b"SQLite format 3\0" {
        return Err(SnapshotError::InvalidHeader);
    }
    drop(f);

    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    // query_only affects this connection, not persistent journal state.
    conn.pragma_update(None, "query_only", "ON")?;
    Ok(conn)
}

/// Read-only schema and row-count inspection; not a substitute for a replay
/// digest or for testing the migration on a restored full backup.
pub fn inspect_connection(conn: &Connection) -> Result<SnapshotReport, SnapshotError> {
    if !has_table(conn, "schema_version")? {
        return Err(SnapshotError::MissingTable("schema_version"));
    }
    let version: Option<i64> =
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })?;
    let version = version.unwrap_or(0);
    if version != LEGACY_SCHEMA_VERSION {
        return Err(SnapshotError::UnsupportedSchema(version));
    }

    for &table in REQUIRED_TABLES {
        if !has_table(conn, table)? {
            return Err(SnapshotError::MissingTable(table));
        }
    }

    for &(table, columns) in REQUIRED_COLUMNS {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info('{table}')"))?;
        let mut found = std::collections::HashSet::new();
        let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for name in names {
            found.insert(name?);
        }
        for &column in columns {
            if !found.contains(column) {
                return Err(SnapshotError::MissingColumn(table, column));
            }
        }
    }

    validate_live_name_index(conn)?;

    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result != "ok" {
        return Err(SnapshotError::IntegrityFailure);
    }
    let mut fk = conn.prepare("PRAGMA foreign_key_check")?;
    if fk.query([])?.next()?.is_some() {
        return Err(SnapshotError::ForeignKeyFailure);
    }

    for sql in [
        "SELECT COUNT(*) FROM folders WHERE id NOT LIKE 'd:%' OR (parent_id != '' AND parent_id NOT LIKE 'd:%')",
        "SELECT COUNT(*) FROM files WHERE msg_id <= 0 OR (parent_id != '' AND parent_id NOT LIKE 'd:%')",
        "SELECT COUNT(*) FROM dirents WHERE (object_kind='file' AND object_id NOT LIKE 'f:%') OR (object_kind='folder' AND object_id NOT LIKE 'd:%') OR (parent_id != '' AND parent_id NOT LIKE 'd:%')",
    ] {
        let invalid: i64 = conn.query_row(sql, [], |row| row.get(0))?;
        if invalid > 0 {
            return Err(SnapshotError::InvalidObjectIds);
        }
    }

    let mut table_counts = Vec::with_capacity(REQUIRED_TABLES.len());
    for &table in REQUIRED_TABLES {
        // SQL identifier is a compile-time fixed internal allowlist, never user input.
        let sql = format!("SELECT COUNT(*) FROM \"{table}\"");
        let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
        table_counts.push((table, count));
    }
    Ok(SnapshotReport {
        schema_version: version,
        table_counts,
    })
}

fn validate_live_name_index(conn: &Connection) -> Result<(), SnapshotError> {
    let index: Option<(i64, i64)> = conn
        .query_row(
            "SELECT [unique], partial FROM pragma_index_list('dirents')
             WHERE name='idx_dirents_live_sibling_name'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if index != Some((1, 1)) {
        return Err(SnapshotError::InvalidIndex);
    }

    let mut statement = conn.prepare(
        "SELECT name, coll FROM pragma_index_xinfo('idx_dirents_live_sibling_name')
         WHERE key=1 ORDER BY seqno",
    )?;
    let columns = statement
        .query_map([], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if columns.len() != 3
        || !columns.iter().zip(["channel_id", "parent_id", "name_key"]).all(
            |((name, collation), expected)| {
                name.as_deref() == Some(expected) && collation.eq_ignore_ascii_case("BINARY")
            },
        )
    {
        return Err(SnapshotError::InvalidIndex);
    }

    // Require the exact Go-v16 active-row predicate, ignoring whitespace,
    // case and one enclosing pair of parentheses. Reject unfamiliar schemas.
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='index'
         AND name='idx_dirents_live_sibling_name' AND tbl_name='dirents'",
        [],
        |row| row.get(0),
    )?;
    let normalized: String = sql
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    let predicate = normalized
        .strip_prefix("createuniqueindexidx_dirents_live_sibling_nameondirents(channel_id,parent_id,name_key)where")
        .map(|predicate| predicate.trim_end_matches(';'));
    if !matches!(predicate, Some("tombstoned=0" | "(tombstoned=0)")) {
        return Err(SnapshotError::InvalidIndex);
    }
    Ok(())
}

fn has_table(conn: &Connection, name: &str) -> Result<bool, SnapshotError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_or_unmigrated_database_without_mutating_it() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(matches!(
            inspect_connection(&conn),
            Err(SnapshotError::MissingTable("schema_version"))
        ));
        conn.execute_batch("CREATE TABLE schema_version(version INTEGER PRIMARY KEY);")
            .unwrap();
        assert!(matches!(
            inspect_connection(&conn),
            Err(SnapshotError::UnsupportedSchema(0))
        ));
    }

    #[test]
    fn rejects_future_version_even_if_other_tables_are_missing() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version(version INTEGER PRIMARY KEY); \
             INSERT INTO schema_version(version) VALUES(17);",
        )
        .unwrap();
        assert!(matches!(
            inspect_connection(&conn),
            Err(SnapshotError::UnsupportedSchema(17))
        ));
    }
}

#[cfg(test)]
mod index_integrity_tests {
    use super::*;

    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE schema_version(version INTEGER PRIMARY KEY);
             INSERT INTO schema_version VALUES(16);
             CREATE TABLE files(channel_id INTEGER,msg_id INTEGER,parent_id TEXT,
                content_msg_id INTEGER,revision INTEGER,encrypted INTEGER,upload_uuid TEXT);
             CREATE TABLE folders(channel_id INTEGER,id TEXT,parent_id TEXT,revision INTEGER,tombstoned INTEGER);
             CREATE TABLE replay_log(channel_id INTEGER,msg_id INTEGER,raw_header TEXT,first_seen_hash TEXT);
             CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,parent_id TEXT,name_key TEXT,
                revision INTEGER,tombstoned INTEGER,object_kind TEXT);
             CREATE TABLE file_revisions(channel_id INTEGER,file_msg_id INTEGER,revision INTEGER,content_msg_id INTEGER);",
        ).unwrap();
        for &table in REQUIRED_TABLES {
            if !has_table(&db, table).unwrap() {
                db.execute(&format!("CREATE TABLE {table}(dummy INTEGER)"), []).unwrap();
            }
        }
        db
    }

    #[test]
    fn index_name_alone_cannot_attest_live_sibling_uniqueness() {
        for definition in [
            "CREATE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key)",
            "CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key)",
            "CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,object_id) WHERE tombstoned=0",
            "CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key) WHERE tombstoned=1",
            "CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key) WHERE tombstoned=1 --where tombstoned=0",
            "CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON files(channel_id,parent_id,msg_id) WHERE revision=1",
        ] {
            let db = fixture();
            db.execute_batch(definition).unwrap();
            assert!(matches!(inspect_connection(&db), Err(SnapshotError::InvalidIndex)), "{definition}");
        }
        let db = fixture();
        db.execute_batch("CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key) WHERE tombstoned = 0").unwrap();
        assert_eq!(inspect_connection(&db).unwrap().schema_version, 16);
    }
}
