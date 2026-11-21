#![cfg(unix)]

use rusqlite::{Connection, OpenFlags};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_storage::{
    SnapshotError, create_consistent_snapshot, inspect_snapshot, snapshot_table_digests,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;

        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let folder = env::temp_dir().join(format!(
            "tdrive-wal-backup-{}-{now}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&folder).unwrap();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
        Self(folder)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn go_fixture() -> Option<PathBuf> {
    env::var_os("TDRIVE_GO_SQLITE_FIXTURE").map(PathBuf::from)
}

#[test]
fn snapshots_a_live_wal_file_including_committed_wal_header_pages() {
    let Some(go_fixture) = go_fixture() else {
        eprintln!("Go synthetic fixture is provided by the CI fixture stage");
        return;
    };
    let scratch = Scratch::new();
    let source = scratch.file("live.sqlite");
    fs::copy(go_fixture, &source).unwrap();

    let source_writer = Connection::open(&source).unwrap();
    let mode: String = source_writer
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "delete");
    source_writer
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    source_writer
        .pragma_update(None, "user_version", 73_i64)
        .unwrap();
    // Keep source writer open; the last WAL checkpoint must not be required.
    assert!(Path::new(&format!("{}-wal", source.display())).exists());

    let original_data = fs::read(&source).unwrap();
    let destination = scratch.file("backup.sqlite");
    let receipt = create_consistent_snapshot(&source, &destination).unwrap();
    assert_eq!(receipt.schema_version, 16);
    assert!(receipt.byte_size > 4096);
    assert!(
        receipt
            .table_counts
            .iter()
            .any(|(name, count)| *name == "files" && *count > 0)
    );
    assert_eq!(
        fs::read(&source).unwrap(),
        original_data,
        "source main file modified"
    );

    let output =
        Connection::open_with_flags(&destination, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let version: i64 = output
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 73, "committed WAL pages were omitted");
    assert_eq!(inspect_snapshot(&destination).unwrap().schema_version, 16);
    let manifest = fs::read_to_string(receipt.manifest_path).unwrap();
    assert!(manifest.contains(&format!("sha256\t{}", receipt.sha256)));
    assert!(manifest.contains(&format!("byte_size\t{}", receipt.byte_size)));
    assert_eq!(snapshot_table_digests(&destination).unwrap().len(), 14);
}

#[test]
fn never_overwrites_existing_snapshot_or_manifest() {
    let Some(go_fixture) = go_fixture() else {
        return;
    };
    let scratch = Scratch::new();
    let destination = scratch.file("backup.sqlite");
    fs::write(&destination, b"existing user backup").unwrap();
    assert!(matches!(
        create_consistent_snapshot(&go_fixture, &destination),
        Err(SnapshotError::BackupOutputExists)
    ));
    assert_eq!(fs::read(&destination).unwrap(), b"existing user backup");

    fs::remove_file(&destination).unwrap();
    fs::write(scratch.file("backup.sqlite.manifest"), b"existing manifest").unwrap();
    assert!(matches!(
        create_consistent_snapshot(&go_fixture, &destination),
        Err(SnapshotError::BackupOutputExists)
    ));
    assert!(!destination.exists());
}

#[test]
fn symlink_source_and_insecure_parent_are_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let Some(go_fixture) = go_fixture() else {
        return;
    };
    let scratch = Scratch::new();
    let symlink_source = scratch.file("source-link");
    symlink(&go_fixture, &symlink_source).unwrap();
    assert!(matches!(
        create_consistent_snapshot(&symlink_source, &scratch.file("out.sqlite")),
        Err(SnapshotError::UnsafePath)
    ));

    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        create_consistent_snapshot(&go_fixture, &scratch.file("out.sqlite")),
        Err(SnapshotError::UnsafeBackupDestination)
    ));
}

#[test]
fn verifies_persisted_manifest_before_restoring_to_new_clone() {
    let Some(source) = go_fixture() else {
        return;
    };
    let scratch = Scratch::new();
    let backup = scratch.file("source-backup.sqlite");
    let original = create_consistent_snapshot(&source, &backup).unwrap();
    let verified = tdrive_storage::verify_snapshot(&backup).unwrap();
    assert_eq!(verified.sha256, original.sha256);
    assert_eq!(verified.table_counts, original.table_counts);

    let clone = scratch.file("restored-dryrun.sqlite");
    let result = tdrive_storage::restore_verified_clone(&backup, &clone).unwrap();
    assert_eq!(result, clone);
    assert_eq!(fs::read(&backup).unwrap(), fs::read(&clone).unwrap());
    assert!(tdrive_storage::restore_verified_clone(&backup, &clone).is_err());
    assert_eq!(
        tdrive_storage::inspect_snapshot(&clone)
            .unwrap()
            .schema_version,
        16
    );
}

#[test]
fn refuses_corrupt_manifest_and_never_creates_clone() {
    let Some(source) = go_fixture() else {
        return;
    };
    let scratch = Scratch::new();
    let backup = scratch.file("backup.sqlite");
    let receipt = create_consistent_snapshot(&source, &backup).unwrap();
    let clone = scratch.file("clone.sqlite");
    fs::write(receipt.manifest_path, b"tdrive-snapshot-v1\nsha256\t0\n").unwrap();
    assert!(matches!(
        tdrive_storage::restore_verified_clone(&backup, &clone),
        Err(SnapshotError::InvalidBackupManifest)
    ));
    assert!(!clone.exists());
    assert!(tdrive_storage::verify_snapshot(&backup).is_err());
}
