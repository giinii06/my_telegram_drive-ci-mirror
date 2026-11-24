//! Actual-binary checks for Go-free, read-only SQLite v16 folder listing.
//! Synthetic fixture tests do not establish real Telegram or production parity.
use rusqlite::{Connection, params};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);
const CHANNEL: i64 = 9_007_199_254_740_993;
const FILE_ID: i64 = 9_223_372_036_854_775_806;

struct Fixture {
    dir: PathBuf,
    db: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be initialized")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tdrive-folder-cli-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).expect("create unique synthetic test folder");
        let db = dir.join("snapshot.sqlite");
        let conn = Connection::open(&db).expect("create synthetic SQLite fixture");
        conn.execute_batch(
            "CREATE TABLE schema_version(version INTEGER PRIMARY KEY);
             INSERT INTO schema_version(version) VALUES(16);
             CREATE TABLE channels(channel_id INTEGER);
             CREATE TABLE replay_log(channel_id INTEGER,msg_id INTEGER,
                 raw_header TEXT,first_seen_hash TEXT);
             CREATE TABLE replay_log_tamper(id INTEGER);
             CREATE TABLE replay_log_rejects(id INTEGER);
             CREATE TABLE folders(channel_id INTEGER,id TEXT,parent_id TEXT,
                 revision INTEGER,tombstoned INTEGER);
             CREATE TABLE files(channel_id INTEGER,msg_id INTEGER,parent_id TEXT,
                 content_msg_id INTEGER,revision INTEGER,encrypted INTEGER,
                 upload_uuid TEXT,tombstoned INTEGER,content_hash TEXT,
                 part_count INTEGER,size INTEGER,upload_time INTEGER,
                 uploader_user_id INTEGER,plaintext_size INTEGER);
             CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,parent_id TEXT,
                 name_key TEXT,revision INTEGER,tombstoned INTEGER,
                 display_name TEXT,object_kind TEXT);
             CREATE TABLE file_revisions(channel_id INTEGER,file_msg_id INTEGER,
                 revision INTEGER,content_msg_id INTEGER);
             CREATE TABLE projection_operations(id INTEGER);
             CREATE TABLE encryption(id INTEGER);
             CREATE TABLE file_parts(id INTEGER);
             CREATE TABLE pending_joins(id INTEGER);
             CREATE TABLE trash_entries(id INTEGER);
             CREATE UNIQUE INDEX idx_dirents_live_sibling_name
                 ON dirents(channel_id,parent_id,name_key)
                 WHERE tombstoned=0;"
        ).expect("initialize detached v16 test schema");
        conn.execute(
            "INSERT INTO folders(channel_id,id,parent_id,revision,tombstoned)
             VALUES(?1,'d:docs','',2,0)",
            params![CHANNEL],
        ).unwrap();
        conn.execute(
            "INSERT INTO dirents(channel_id,object_id,parent_id,name_key,
             revision,tombstoned,display_name,object_kind)
             VALUES(?1,'d:docs','','docs',2,0,?2,'folder')",
            params![CHANNEL, "가이드\t\"2026\"\n"],
        ).unwrap();
        conn.execute(
            "INSERT INTO files(channel_id,msg_id,parent_id,content_msg_id,
             revision,encrypted,upload_uuid,tombstoned,content_hash,
             part_count,size,upload_time,uploader_user_id,plaintext_size)
             VALUES(?1,?2,'',?3,1,1,'uuid',0,'sha',1,42,1700000000,77,40)",
            params![CHANNEL, FILE_ID, FILE_ID - 1],
        ).unwrap();
        conn.execute(
            "INSERT INTO dirents(channel_id,object_id,parent_id,name_key,
             revision,tombstoned,display_name,object_kind)
             VALUES(?1,?2,'','f-name',1,0,'한국어 résumé','file')",
            params![CHANNEL, format!("f:{FILE_ID}")],
        ).unwrap();
        drop(conn);
        Self { dir, db }
    }

    fn run(&self, channel: &str, parent: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
            .args(["--list-folder", self.db.to_str().unwrap(), channel, parent])
            .output()
            .expect("launch actual Rust inspector")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn lists_rust_json_with_lossless_ids_unicode_and_escaping() {
    let fixture = Fixture::new();
    let before = fs::read(&fixture.db).unwrap();
    let output = fixture.run(&CHANNEL.to_string(), "");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let rendered = String::from_utf8(output.stdout).unwrap();
    assert!(rendered.starts_with(r#"{"folders":[{"id":"d:docs""#), "{rendered}");
    assert!(rendered.contains(r#""name":"가이드\t\"2026\"\n""#), "{rendered}");
    assert!(rendered.contains(r#""msgId":"9223372036854775806""#), "{rendered}");
    assert!(rendered.contains(r#""name":"한국어 résumé""#), "{rendered}");
    assert_eq!(fs::read(&fixture.db).unwrap(), before, "snapshot must not be altered");
}

#[test]
fn rejects_nonpositive_nondecimal_overflow_and_missing_database() {
    let fixture = Fixture::new();
    for invalid in ["0", "-7", "1.5", "9223372036854775808"] {
        let output = fixture.run(invalid, "");
        assert!(!output.status.success(), "accepted invalid channel {invalid}");
        assert!(output.stdout.is_empty(), "partial JSON must not be emitted");
    }
    let missing = fixture.dir.join("missing.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .args(["--list-folder", missing.to_str().unwrap(), "123", ""])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!missing.exists(), "missing DB must not be created");
}

#[test]
fn rejects_unsupported_schema_and_wal_sidecar() {
    let fixture = Fixture::new();
    let conn = Connection::open(&fixture.db).unwrap();
    conn.execute("UPDATE schema_version SET version=17", []).unwrap();
    drop(conn);
    assert!(!fixture.run(&CHANNEL.to_string(), "").status.success());

    let conn = Connection::open(&fixture.db).unwrap();
    conn.execute("UPDATE schema_version SET version=16", []).unwrap();
    drop(conn);
    let sidecar = PathBuf::from(format!("{}-wal", fixture.db.display()));
    fs::write(&sidecar, b"not a safe detached snapshot").unwrap();
    let output = fixture.run(&CHANNEL.to_string(), "");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn never_follows_final_symlink_or_accepts_relative_snapshot() {
    let fixture = Fixture::new();
    let relative = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .args(["--list-folder", "relative.sqlite", "123", ""])
        .output()
        .unwrap();
    assert!(!relative.status.success());
    #[cfg(unix)]
    {
        let link = fixture.dir.join("linked.sqlite");
        std::os::unix::fs::symlink(&fixture.db, &link).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
            .args(["--list-folder", link.to_str().unwrap(), "123", ""])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn multipart_cli_outputs_complete_ordered_metadata_without_writing_snapshot() {
    let fixture = Fixture::new();
    let conn = Connection::open(&fixture.db).unwrap();
    conn.execute_batch(
        "DROP TABLE file_parts;
         CREATE TABLE file_parts(channel_id INTEGER,upload_uuid TEXT,part_index INTEGER,msg_id INTEGER,size INTEGER);
         INSERT INTO file_parts VALUES(9007199254740993,'uuid',0,9223372036854775805,42);",
    ).unwrap();
    drop(conn);
    let run = || Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .args(["--list-parts", fixture.db.to_str().unwrap(), &CHANNEL.to_string(), &FILE_ID.to_string()])
        .output().unwrap();
    let before = fs::read(&fixture.db).unwrap();
    let result = run();
    assert!(result.status.success());
    assert_eq!(String::from_utf8(result.stdout).unwrap().trim(),
        r#"{"parts":[{"partIndex":"0","msgId":"9223372036854775805","size":"42"}]}"#);
    assert_eq!(fs::read(&fixture.db).unwrap(), before);
    let conn = Connection::open(&fixture.db).unwrap();
    conn.execute("DELETE FROM file_parts", []).unwrap();
    drop(conn);
    let before = fs::read(&fixture.db).unwrap();
    let result = run();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert_eq!(fs::read(&fixture.db).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn offline_assembly_publishes_only_verified_complete_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.dir, fs::Permissions::from_mode(0o700)).unwrap();
    let conn = Connection::open(&fixture.db).unwrap();
    conn.execute_batch(
        "DROP TABLE file_parts;
         CREATE TABLE file_parts(channel_id INTEGER,upload_uuid TEXT,part_index INTEGER,msg_id INTEGER,size INTEGER);
         UPDATE files SET part_count=2,size=5;
         INSERT INTO file_parts VALUES(9007199254740993,'uuid',0,10,2),(9007199254740993,'uuid',1,11,3);",
    ).unwrap();
    drop(conn);
    for (name, bytes) in [("10.part", b"ab".as_slice()), ("11.part", b"cde".as_slice())] {
        let path = fixture.dir.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let target = fixture.dir.join("joined");
    let hash = "36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c";
    let run = |digest: &str| Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg("--assemble-parts").arg(&fixture.db).arg(CHANNEL.to_string()).arg(FILE_ID.to_string())
        .arg(&fixture.dir).arg(&target).arg(digest).output().unwrap();
    let before = fs::read(&fixture.db).unwrap();
    assert!(!run(&"0".repeat(64)).status.success());
    assert!(!target.exists());
    assert!(run(hash).status.success());
    assert_eq!(fs::read(&target).unwrap(), b"abcde");
    assert!(!run(hash).status.success());
    assert_eq!(fs::read(&target).unwrap(), b"abcde");
    fs::remove_file(&target).unwrap();
    fs::remove_file(fixture.dir.join("11.part")).unwrap();
    assert!(!run(hash).status.success());
    assert!(!target.exists());
    assert_eq!(fs::read(&fixture.db).unwrap(), before);
    assert!(fs::read_dir(&fixture.dir).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".tdrive-assembly-")));
}

#[cfg(unix)]
#[test]
fn backup_cli_creates_verifies_and_restores_without_overwriting() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.dir, fs::Permissions::from_mode(0o700)).unwrap();
    let backup = fixture.dir.join("backup.sqlite");
    let manifest = fixture.dir.join("backup.sqlite.manifest");
    let restored = fixture.dir.join("restored.sqlite");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
            .args(args)
            .output()
            .unwrap()
    };
    let source = fixture.db.to_str().unwrap();
    let dest = backup.to_str().unwrap();
    let clone = restored.to_str().unwrap();
    let before = fs::read(&fixture.db).unwrap();
    assert!(run(&["--create-backup", source, dest]).status.success());
    assert!(manifest.exists());
    assert_eq!(fs::metadata(&backup).unwrap().permissions().mode() & 0o077, 0);
    assert!(run(&["--verify-backup", dest]).status.success());
    assert!(run(&["--restore-clone", dest, clone]).status.success());
    assert_eq!(fs::read(&backup).unwrap(), fs::read(&restored).unwrap());
    assert!(!run(&["--restore-clone", dest, clone]).status.success());
    assert!(!run(&["--create-backup", source, dest]).status.success());
    assert_eq!(fs::read(&fixture.db).unwrap(), before);
    fs::remove_file(&restored).unwrap();
    fs::write(&manifest, b"corrupt").unwrap();
    assert!(!run(&["--verify-backup", dest]).status.success());
    assert!(!run(&["--restore-clone", dest, clone]).status.success());
    assert!(!restored.exists());
}
