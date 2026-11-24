#![cfg(unix)]
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Guard(PathBuf, Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.1.kill();
        let _ = self.1.wait();
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn authenticated_fixed_media_returns_exact_ranges() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("tdrive-media-http-{}-{now}", std::process::id()));
    DirBuilder::new().mode(0o700).create(&root).unwrap();
    let token = root.join("token");
    let secret = "0123456789abcdefghijklmnopqrstuvwxyz";
    OpenOptions::new().write(true).create_new(true).mode(0o600).open(&token).unwrap()
        .write_all(secret.as_bytes()).unwrap();
    let media = root.join("media");
    fs::write(&media, b"0123456789").unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let child = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg("--serve-media").arg(&media).arg(&token).arg(port.to_string())
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let _guard = Guard(root, child);
    let request = |path: &str, auth: bool, range: &str| {
        let mut socket = (0..100).find_map(|_| {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => Some(s),
                Err(_) => { std::thread::sleep(Duration::from_millis(30)); None }
            }
        }).expect("media listener");
        socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let auth = if auth { format!("Authorization: Bearer {secret}\r\n") } else { String::new() };
        write!(socket, "GET {path} HTTP/1.1\r\nHost: localhost\r\n{auth}{range}\r\n").unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        response
    };
    assert!(request("/v1/media", false, "").starts_with("HTTP/1.1 401"));
    assert!(request("/v1/media?path=/etc/passwd", true, "").starts_with("HTTP/1.1 404"));
    let full = request("/v1/media", true, "");
    assert!(full.starts_with("HTTP/1.1 200"));
    assert!(full.ends_with("\r\n\r\n0123456789"));
    let part = request("/v1/media", true, "Range: bytes=2-4\r\n");
    assert!(part.starts_with("HTTP/1.1 206"));
    assert!(part.contains("Content-Range: bytes 2-4/10\r\n"));
    assert!(part.contains("Content-Length: 3\r\n"));
    assert!(part.ends_with("\r\n\r\n234"));
    let bad = request("/v1/media", true, "Range: bytes=10-\r\n");
    assert!(bad.starts_with("HTTP/1.1 416"));
    assert!(bad.contains("Content-Range: bytes */10\r\n"));
    assert!(bad.ends_with("\r\n\r\n"));
    assert!(request("/v1/media", true, "Range: bytes=0-1\r\nRange: bytes=2-3\r\n").starts_with("HTTP/1.1 400"));
}
