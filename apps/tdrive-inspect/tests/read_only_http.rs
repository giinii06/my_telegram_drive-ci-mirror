#![cfg(unix)]
// Actual Rust binary loopback smoke against disposable SQLite and key files.
// No network beyond localhost, no Telegram or production data.
use rusqlite::Connection;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Cleanup {
    root: PathBuf,
    child: Child,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture() -> (PathBuf, PathBuf, PathBuf) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("tdrive-http-{}-{now}", std::process::id()));
    DirBuilder::new().mode(0o700).create(&root).unwrap();
    let sqlite = root.join("snapshot.sqlite");
    let db = Connection::open(&sqlite).unwrap();
    db.execute_batch(
        "CREATE TABLE schema_version(version INTEGER PRIMARY KEY);
         INSERT INTO schema_version VALUES(16);
         CREATE TABLE channels(channel_id INTEGER);
         CREATE TABLE replay_log(channel_id INTEGER,msg_id INTEGER,raw_header TEXT,first_seen_hash TEXT);
         CREATE TABLE replay_log_tamper(id INTEGER);
         CREATE TABLE replay_log_rejects(id INTEGER);
         CREATE TABLE folders(channel_id INTEGER,id TEXT,parent_id TEXT,revision INTEGER,tombstoned INTEGER);
         CREATE TABLE files(channel_id INTEGER,msg_id INTEGER,parent_id TEXT,content_msg_id INTEGER,
                            revision INTEGER,encrypted INTEGER,upload_uuid TEXT,tombstoned INTEGER);
         CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,parent_id TEXT,name_key TEXT,
                             revision INTEGER,tombstoned INTEGER,display_name TEXT,object_kind TEXT);
         CREATE TABLE file_revisions(channel_id INTEGER,file_msg_id INTEGER,revision INTEGER,content_msg_id INTEGER);
         CREATE TABLE projection_operations(id INTEGER);
         CREATE TABLE encryption(id INTEGER);
         CREATE TABLE file_parts(id INTEGER);
         CREATE TABLE pending_joins(id INTEGER);
         CREATE TABLE trash_entries(id INTEGER);
         CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key)
           WHERE tombstoned=0;"
    ).unwrap();
    drop(db);
    let token = root.join("token");
    let mut file = OpenOptions::new().create_new(true).write(true).mode(0o600).open(&token).unwrap();
    file.write_all(b"0123456789abcdefghijklmnopqrstuvwxyzABCDEF").unwrap();
    (root, sqlite, token)
}

fn request(port: u16, credentials: Option<&str>, path: &str) -> String {
    let mut connection = TcpStream::connect(("127.0.0.1", port)).unwrap();
    connection.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let auth = credentials.map_or(String::new(), |s| format!("Authorization: Bearer {s}\r\n"));
    write!(
        &mut connection,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\n{auth}\r\n"
    ).unwrap();
    let mut response = String::new();
    connection.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn loopback_health_requires_auth_and_offline_folder_reads_remain_readonly() {
    let (root, snapshot, token) = fixture();
    let bytes_before = fs::read(&snapshot).unwrap();
    let available = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = available.local_addr().unwrap().port();
    drop(available);

    let child = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .args([
            "--serve-readonly",
            snapshot.to_str().unwrap(),
            token.to_str().unwrap(),
            &port.to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let guard = Cleanup { root, child };
    let mut ready = false;
    for _ in 0..100 {
        if let Ok(mut probe) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = probe.set_read_timeout(Some(Duration::from_secs(3)));
            if probe.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n").is_ok() {
                let mut response = String::new();
                if probe.read_to_string(&mut response).is_ok()
                    && response.starts_with("HTTP/1.1 401 Unauthorized")
                {
                    ready = true;
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(ready, "Rust HTTP binary did not bind loopback");

    assert!(request(port, None, "/health").starts_with("HTTP/1.1 401 Unauthorized"));
    assert!(request(port, Some("wrong"), "/health").starts_with("HTTP/1.1 401 Unauthorized"));
    let good = "0123456789abcdefghijklmnopqrstuvwxyzABCDEF";
    let health = request(port, Some(good), "/health");
    assert!(health.starts_with("HTTP/1.1 200 OK"), "{health}");
    assert!(health.ends_with(r#"{"status":"ok"}"#), "{health}");

    let files = request(port, Some(good), "/v1/folders?channel=123&parent=");
    assert!(files.starts_with("HTTP/1.1 200 OK"), "{files}");
    assert!(files.ends_with(r#"{"folders":[],"files":[]}"#), "{files}");
    assert_eq!(fs::read(&snapshot).unwrap(), bytes_before);

    drop(guard);
}
