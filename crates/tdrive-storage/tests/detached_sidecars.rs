use rusqlite::Connection;
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tdrive_storage::{SnapshotError, inspect_snapshot};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}
fn scratch() -> Scratch {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let p = std::env::temp_dir().join(format!("tdrive-sidecars-{}-{now}", std::process::id()));
    fs::create_dir(&p).unwrap();
    Scratch(p)
}

#[test]
fn rollback_journal_prevents_detached_snapshot_open() {
    let root = scratch();
    let path = root.0.join("snapshot.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE t(x); BEGIN IMMEDIATE; INSERT INTO t VALUES (1);").unwrap();
    let journal = root.0.join("snapshot.sqlite-journal");
    assert!(journal.exists());
    assert!(matches!(inspect_snapshot(&path), Err(SnapshotError::ActiveWal)));
    conn.execute_batch("ROLLBACK;").unwrap();
}

#[cfg(unix)]
#[test]
fn dangling_sidecar_is_not_treated_as_absent() {
    use std::os::unix::fs::symlink;
    let root = scratch();
    let path = root.0.join("snapshot.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE t(x);").unwrap();
    drop(conn);
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = root.0.join(format!("snapshot.sqlite{suffix}"));
        symlink(root.0.join("absent"), &sidecar).unwrap();
        assert!(!sidecar.exists());
        assert!(matches!(inspect_snapshot(&path), Err(SnapshotError::ActiveWal)));
        fs::remove_file(sidecar).unwrap();
    }
}
