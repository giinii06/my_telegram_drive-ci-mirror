//! Localhost-only, read-only HTTP adapter for detached Go v16 snapshots.
//! No public bind, remote Telegram access, write endpoints or TLS is exposed.
use crate::{folder_json, parts_json};
use crate::http_deadline::remaining;
use crate::http_contract::{ApiRoute, HttpError, parse_request, valid_token};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, Instant};

const MAX_REQUEST: usize = 8192;
const HEADER_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_RESPONSE: usize = 16 * 1024 * 1024;

struct PrivateToken(Vec<u8>);

impl Drop for PrivateToken {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

fn same_private_token(before: &fs::Metadata, current: &fs::Metadata) -> bool {
    current.is_file()
        && current.nlink() == 1
        && current.permissions().mode() & 0o077 == 0
        && current.dev() == before.dev()
        && current.ino() == before.ino()
        && current.len() == before.len()
        && current.mtime() == before.mtime()
        && current.mtime_nsec() == before.mtime_nsec()
        && current.ctime() == before.ctime()
        && current.ctime_nsec() == before.ctime_nsec()
}

fn read_token(path: &Path) -> Result<PrivateToken, &'static str> {
    if !path.is_absolute() {
        return Err("private bearer token path must be absolute");
    }
    let parent = path.parent().ok_or("invalid private token parent")?;
    let dir = fs::symlink_metadata(parent).map_err(|_| "private token directory missing")?;
    if !dir.is_dir()
        || dir.file_type().is_symlink()
        || dir.permissions().mode() & 0o077 != 0
    {
        return Err("bearer token directory must be private");
    }
    let before = fs::symlink_metadata(path).map_err(|_| "private bearer token unavailable")?;
    if !before.is_file()
        || before.file_type().is_symlink()
        || before.nlink() != 1
        || before.permissions().mode() & 0o077 != 0
        || !(32..=128).contains(&before.len())
    {
        return Err("private bearer token permissions or size invalid");
    }
    let mut file = File::open(path).map_err(|_| "private bearer token unavailable")?;
    let opened = file.metadata().map_err(|_| "bearer token metadata unavailable")?;
    if !same_private_token(&before, &opened) {
        return Err("private bearer token changed during read");
    }
    let mut buffer = PrivateToken(Vec::new());
    (&mut file)
        .take(129)
        .read_to_end(&mut buffer.0)
        .map_err(|_| "private bearer token unreadable")?;
    let after = file.metadata().map_err(|_| "bearer token metadata unavailable")?;
    let named = fs::symlink_metadata(path).map_err(|_| "private bearer token unavailable")?;
    if !same_private_token(&opened, &after)
        || !same_private_token(&opened, &named)
        || buffer.0.len() as u64 != opened.len()
    {
        return Err("private bearer token changed during read");
    }
    if !valid_token(&buffer.0) {
        return Err("private bearer token must be 32–128 header-safe ASCII bytes");
    }
    Ok(buffer)
}

fn send(socket: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    );
    socket.write_all(headers.as_bytes())?;
    socket.write_all(body.as_bytes())?;
    socket.flush()
}

fn receive(socket: &mut TcpStream) -> Result<Vec<u8>, HttpError> {
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    let started = Instant::now();
    loop {
        if request.len() >= MAX_REQUEST {
            return Err(HttpError::TooLarge);
        }
        let left = remaining(started, Instant::now(), HEADER_TIMEOUT)
            .ok_or(HttpError::BadRequest)?;
        socket
            .set_read_timeout(Some(left))
            .map_err(|_| HttpError::BadRequest)?;
        let size = socket.read(&mut chunk).map_err(|_| HttpError::BadRequest)?;
        if remaining(started, Instant::now(), HEADER_TIMEOUT).is_none() {
            return Err(HttpError::BadRequest);
        }
        if size == 0 {
            return Err(HttpError::BadRequest);
        }
        request.extend_from_slice(&chunk[..size]);
        if request.len() > MAX_REQUEST {
            return Err(HttpError::TooLarge);
        }
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(request);
        }
    }
}

fn handle(socket: &mut TcpStream, snapshot: &Path, secret: &[u8]) {
    let _ = socket.set_write_timeout(Some(Duration::from_secs(3)));
    let route = receive(socket).and_then(|raw| parse_request(&raw, secret));
    let (status, response) = match route {
        Ok(ApiRoute::Health) => ("200 OK", r#"{"status":"ok"}"#.to_string()),
        Ok(ApiRoute::Parts { channel_id, message_id }) => {
            match tdrive_storage::resolve_multipart_snapshot(snapshot, channel_id, message_id) {
                Ok(Some(parts)) => {
                    let json = parts_json::encode_json(&parts);
                    if json.len() > MAX_RESPONSE {
                        ("413 Content Too Large", r#"{"error":"response too large"}"#.to_string())
                    } else {
                        ("200 OK", json)
                    }
                }
                Ok(None) => ("404 Not Found", r#"{"error":"not found"}"#.to_string()),
                Err(_) => ("422 Unprocessable Content", r#"{"error":"invalid or incomplete snapshot"}"#.to_string()),
            }
        }
        Ok(ApiRoute::Folder { channel_id, parent }) => {
            match tdrive_storage::list_folder_contents_snapshot_bounded(snapshot, channel_id, &parent, 10_000, 1024 * 1024) {
                Ok(listing) => {
                    let json = folder_json::encode_json(&listing);
                    if json.len() > MAX_RESPONSE {
                        ("413 Content Too Large", r#"{"error":"response too large"}"#.to_string())
                    } else {
                        ("200 OK", json)
                    }
                }
                Err(tdrive_storage::SnapshotError::SnapshotTooLarge) => ("413 Content Too Large", r#"{"error":"response too large"}"#.to_string()),
                Err(_) => ("422 Unprocessable Content", r#"{"error":"invalid snapshot"}"#.to_string()),
            }
        }
        Err(HttpError::Unauthorized) => ("401 Unauthorized", r#"{"error":"unauthorized"}"#.to_string()),
        Err(HttpError::NotFound) => ("404 Not Found", r#"{"error":"not found"}"#.to_string()),
        Err(HttpError::MethodNotAllowed) => ("405 Method Not Allowed", r#"{"error":"method denied"}"#.to_string()),
        Err(HttpError::TooLarge) => ("431 Request Header Fields Too Large", r#"{"error":"headers too large"}"#.to_string()),
        Err(HttpError::BadRequest) => ("400 Bad Request", r#"{"error":"bad request"}"#.to_string()),
    };
    let _ = send(socket, status, &response);
}

/// Bind only the loopback IPv4 address, with fixed snapshot and private token.
/// Sequential handling is intentional: no detached thread or unbounded queue.
pub fn serve(snapshot: &Path, key: &Path, port: u16) -> Result<(), &'static str> {
    if port == 0 || !snapshot.is_absolute() {
        return Err("loopback port and absolute detached snapshot required");
    }
    let secret = read_token(key)?;
    tdrive_storage::inspect_snapshot(snapshot).map_err(|_| "invalid detached SQLite snapshot")?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|_| "cannot bind loopback socket")?;
    println!("Rust read-only snapshot API listening on 127.0.0.1:{port}");
    for mut socket in listener.incoming().flatten() {
        handle(&mut socket, snapshot, &secret.0);
    }
    Ok(())
}

#[cfg(test)]
mod token_tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

    #[test]
    fn token_metadata_comparison_rejects_same_inode_permission_changes() {
        let root = std::env::temp_dir().join(format!(
            "tdrive-token-{}-{:?}", std::process::id(), Instant::now()
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let path = root.join("token");
        let mut file = fs::OpenOptions::new().create_new(true).write(true).mode(0o600).open(&path).unwrap();
        file.write_all(b"0123456789abcdefghijklmnopqrstuvwxyz").unwrap();
        let before = file.metadata().unwrap();
        assert!(same_private_token(&before, &file.metadata().unwrap()));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!same_private_token(&before, &file.metadata().unwrap()));
        fs::remove_dir_all(root).unwrap();
    }
}
