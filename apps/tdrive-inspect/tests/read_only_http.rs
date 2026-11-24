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
                            revision INTEGER,encrypted INTEGER,upload_uuid TEXT,tombstoned INTEGER,
                            content_hash TEXT,part_count INTEGER,size INTEGER,upload_time INTEGER,
                            uploader_user_id INTEGER,plaintext_size INTEGER);
         CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,parent_id TEXT,name_key TEXT,
                             revision INTEGER,tombstoned INTEGER,display_name TEXT,object_kind TEXT);
         CREATE TABLE file_revisions(channel_id INTEGER,file_msg_id INTEGER,revision INTEGER,content_msg_id INTEGER);
         CREATE TABLE projection_operations(id INTEGER);
         CREATE TABLE encryption(id INTEGER);
         CREATE TABLE file_parts(channel_id INTEGER,upload_uuid TEXT,part_index INTEGER,msg_id INTEGER,size INTEGER);
         INSERT INTO files(channel_id,msg_id,parent_id,content_msg_id,revision,encrypted,upload_uuid,tombstoned,content_hash,part_count,size,upload_time,uploader_user_id,plaintext_size)
         VALUES(123,10,'',0,1,0,'complete',0,'',2,5,0,0,0),
               (123,11,'',0,1,0,'incomplete',0,'',2,5,0,0,0),
               (123,12,'',0,1,0,'deleted',1,'',1,3,0,0,0);
         INSERT INTO file_parts VALUES(123,'complete',1,8,3),(123,'complete',0,7,2),
                                      (123,'incomplete',0,7,2),(123,'deleted',0,7,3);
         INSERT INTO folders VALUES(321,'d:large','',1,0);
         INSERT INTO dirents VALUES(321,'d:large','','large',1,0,hex(zeroblob(524289)),'folder');
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
    let parts = request(port, Some(good), "/v1/parts?channel=123&message=10");
    assert!(parts.starts_with("HTTP/1.1 200 OK"), "{parts}");
    assert!(parts.ends_with(r#"{"parts":[{"partIndex":"0","msgId":"7","size":"2"},{"partIndex":"1","msgId":"8","size":"3"}]}"#));
    assert!(request(port, None, "/v1/parts?channel=123&message=10").starts_with("HTTP/1.1 401"));
    for path in ["/v1/parts?channel=999&message=10", "/v1/parts?channel=123&message=12"] {
        assert!(request(port, Some(good), path).starts_with("HTTP/1.1 404"));
    }
    assert!(request(port, Some(good), "/v1/parts?channel=123&message=11").starts_with("HTTP/1.1 422"));
    for path in ["/v1/parts?channel=123", "/v1/parts?channel=123&message=0", "/v1/parts?channel=123&message=10&message=11"] {
        assert!(request(port, Some(good), path).starts_with("HTTP/1.1 400"));
    }
    let oversized = request(port, Some(good), "/v1/folders?channel=321&parent=");
    assert!(oversized.starts_with("HTTP/1.1 413 Content Too Large"), "{oversized}");
    assert!(oversized.ends_with(r#"{"error":"response too large"}"#));
    assert_eq!(fs::read(&snapshot).unwrap(), bytes_before);

    drop(guard);
}

#[test]
fn dav_propfind_is_bounded_authenticated_and_readonly() {
    let (root, snapshot, token) = fixture();
    let db = Connection::open(&snapshot).unwrap();
    db.execute_batch("INSERT INTO channels VALUES(124);
        INSERT INTO folders VALUES(124,'d:docs','',1,0),(124,'d:child','d:docs',1,0);
        INSERT INTO dirents VALUES(124,'d:docs','','a & b',1,0,'A & B','folder'),
           (124,'d:child','d:docs','child',1,0,'Child','folder');").unwrap();
    drop(db);
    let bytes = fs::read(&snapshot).unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let child = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .args(["--serve-readonly",snapshot.to_str().unwrap(),token.to_str().unwrap(),&port.to_string()])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let _guard = Cleanup {root, child};
    let call = |method: &str, path: &str, depth: &str, auth: bool| {
        let mut socket = (0..100).find_map(|_| match TcpStream::connect(("127.0.0.1",port)) {
            Ok(s) => Some(s),
            Err(_) => {std::thread::sleep(Duration::from_millis(30)); None}
        }).unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let auth = if auth {"Authorization: Bearer 0123456789abcdefghijklmnopqrstuvwxyzABCDEF\r\n"} else {""};
        write!(socket,"{method} {path} HTTP/1.1\r\nHost: localhost\r\n{auth}{depth}\r\n").unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        response
    };
    assert!(call("PROPFIND","/dav/124/","Depth: 1\r\n",false).starts_with("HTTP/1.1 401"));
    let root = call("PROPFIND","/dav/124/","Depth: 1\r\nContent-Length: 0\r\n",true);
    assert!(root.starts_with("HTTP/1.1 207"));
    assert!(root.contains("/dav/124/A%20%26%20B/"));
    assert!(root.contains("A &amp; B"));
    let child = call("PROPFIND","/dav/124/A%20%26%20B/","Depth: 1\r\n",true);
    assert!(child.contains("/dav/124/A%20%26%20B/Child/"));
    assert!(call("PROPFIND","/dav/124/","Depth: 0\r\n",true).starts_with("HTTP/1.1 207"));
    assert!(call("PROPFIND","/dav/999/","Depth: 1\r\n",true).starts_with("HTTP/1.1 404"));
    assert!(call("PROPFIND","/dav/124/%2e%2e/","Depth: 1\r\n",true).starts_with("HTTP/1.1 400"));
    assert!(call("PROPFIND","/dav/124/","Depth: infinity\r\n",true).starts_with("HTTP/1.1 400"));
    assert!(call("PUT","/dav/124/","",true).starts_with("HTTP/1.1 405"));
    assert_eq!(fs::read(&snapshot).unwrap(), bytes);
}
