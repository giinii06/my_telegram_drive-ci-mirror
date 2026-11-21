//! WAL-aware Go-v16 SQLite snapshots using SQLite's online backup API.
//!
//! Never copy a WAL database with a raw file copy. No live database or
//! manifest is ever replaced. The caller provides a private output directory.

use crate::{SnapshotError, SnapshotReport, inspect_connection, inspect_snapshot};
use rusqlite::{Connection, OpenFlags, backup::Backup};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

static SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct BackupReceipt {
    pub snapshot_path: PathBuf,
    pub manifest_path: PathBuf,
    pub sha256: String,
    pub byte_size: u64,
    pub schema_version: i64,
    pub table_counts: Vec<(&'static str, i64)>,
}

struct TempGuard(PathBuf);

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.0);
        }
    }
}

fn ensure_new_output(path: &Path) -> Result<(), SnapshotError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(SnapshotError::BackupOutputExists),
        Err(error) => Err(error.into()),
    }
}

fn validate_source(source: &Path) -> Result<Connection, SnapshotError> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(SnapshotError::UnsafePath);
    }
    let mut header = [0u8; 16];
    File::open(source)?.read_exact(&mut header)?;
    if &header != b"SQLite format 3\0" {
        return Err(SnapshotError::InvalidHeader);
    }
    // Unlike inspect_snapshot, WAL + SHM sidecars are intentionally allowed
    // because the SQLite online-backup API reads committed WAL pages safely.
    let connection = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.pragma_update(None, "query_only", "ON")?;
    inspect_connection(&connection)?;
    Ok(connection)
}

fn ensure_private_backup_parent(parent: &Path) -> Result<(), SnapshotError> {
    let metadata = fs::symlink_metadata(parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(SnapshotError::UnsafeBackupDestination);
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(SnapshotError::UnsafeBackupDestination);
    }
    #[cfg(not(unix))]
    return Err(SnapshotError::UnsafeBackupDestination);
    #[cfg(unix)]
    Ok(())
}

fn fresh_temp(parent: &Path) -> Result<(File, TempGuard), SnapshotError> {
    let sequence = SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SnapshotError::UnsafeBackupDestination)?
        .as_nanos();
    let path = parent.join(format!(
        ".tdrive-snapshot-{}-{nanos}-{sequence}.tmp",
        std::process::id()
    ));
    #[cfg(unix)]
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    #[cfg(not(unix))]
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&path)?;
    Ok((file, TempGuard(path)))
}

fn digest_file(path: &Path) -> Result<(String, u64), SnapshotError> {
    let mut f = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut bytes = 0u64;
    loop {
        let count = f.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        digest.update(&buffer[..count]);
    }
    Ok((format!("{:x}", digest.finalize()), bytes))
}

/// Produces a *new*, consistent, 0600 Go-v16 SQLite backup and a small
/// nonsecret SHA-256 manifest; never overwrites any existing destination.
///
/// Requires destination parent to be a trusted private 0700 Unix directory.
/// Backing up a live WAL source is supported without raw-copied sidecars.
/// Original Go daemon remains responsible for live writer ownership.
pub fn create_consistent_snapshot(
    source: &Path,
    destination: &Path,
) -> Result<BackupReceipt, SnapshotError> {
    let parent = destination
        .parent()
        .ok_or(SnapshotError::UnsafeBackupDestination)?;
    ensure_private_backup_parent(parent)?;
    let name = destination
        .file_name()
        .ok_or(SnapshotError::UnsafeBackupDestination)?;
    let manifest = parent.join(format!("{}.manifest", name.to_string_lossy()));
    ensure_new_output(destination)?;
    ensure_new_output(&manifest)?;
    let original = validate_source(source)?;

    let (file, temp) = fresh_temp(parent)?;
    drop(file);
    let mut target = Connection::open_with_flags(
        &temp.0,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )?;
    {
        let backup = Backup::new(&original, &mut target)?;
        backup.run_to_completion(64, Duration::from_millis(20), None)?;
    }
    // Normalize the self-contained output, removing WAL-mode artifacts
    // before inspecting the completed backup as a detached SQLite file.
    target.pragma_update(None, "journal_mode", "DELETE")?;
    drop(target);
    let SnapshotReport {
        schema_version,
        table_counts,
    } = inspect_snapshot(&temp.0)?;

    let (sha256, byte_size) = digest_file(&temp.0)?;
    File::open(&temp.0)?.sync_all()?;
    // hard_link is a no-clobber atomic creation on the same filesystem.
    // Unlike rename this cannot replace an existing operator backup.
    fs::hard_link(&temp.0, destination)?;
    fs::remove_file(&temp.0)?;
    File::open(parent)?.sync_all()?;

    let (mut manifest_file, manifest_temp) = fresh_temp(parent)?;
    let mut contents = format!(
        "tdrive-snapshot-v1\nschema_version\t{schema_version}\nbyte_size\t{byte_size}\nsha256\t{sha256}\n"
    );
    for (table, count) in &table_counts {
        contents.push_str(&format!("table\t{table}\t{count}\n"));
    }
    manifest_file.write_all(contents.as_bytes())?;
    manifest_file.sync_all()?;
    drop(manifest_file);
    fs::hard_link(&manifest_temp.0, &manifest)?;
    fs::remove_file(&manifest_temp.0)?;
    File::open(parent)?.sync_all()?;

    Ok(BackupReceipt {
        snapshot_path: destination.to_path_buf(),
        manifest_path: manifest,
        sha256,
        byte_size,
        schema_version,
        table_counts,
    })
}
