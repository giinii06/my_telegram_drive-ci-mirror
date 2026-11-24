//! Verify stored bytes before publishing into a caller-owned private directory.
//! The caller must exclusively own the staged file and destination directory.
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedContent {
    size: u64,
    digest: [u8; 32],
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid expected content")
}

impl ExpectedContent {
    pub fn size(self) -> u64 {
        self.size
    }

    pub fn parse(size: &str, digest: &str) -> io::Result<Self> {
        if size.is_empty() || !size.bytes().all(|b| b.is_ascii_digit())
            || digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid());
        }
        let size = size.parse::<u64>().map_err(|_| invalid())?;
        if size > i64::MAX as u64 {
            return Err(invalid());
        }
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16)
                .map_err(|_| invalid())?;
        }
        Ok(Self { size, digest: bytes })
    }
}

pub fn validate_private_target(target: &Path) -> io::Result<()> {
    crate::policy::validate_download_path(target).map_err(io::Error::other)?;
    let parent = target.parent().ok_or_else(invalid)?;
    let dir = fs::symlink_metadata(parent)?;
    if dir.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "private destination required"));
    }
    Ok(())
}

fn safe_file(meta: &fs::Metadata, expected: ExpectedContent) -> bool {
    meta.is_file() && meta.nlink() == 1 && meta.permissions().mode() & 0o077 == 0
        && meta.len() == expected.size
}

fn unchanged(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.dev() == b.dev() && a.ino() == b.ino() && a.len() == b.len()
        && a.mtime() == b.mtime() && a.mtime_nsec() == b.mtime_nsec()
        && a.ctime() == b.ctime() && a.ctime_nsec() == b.ctime_nsec()
}

/// Verify a regular private staged file in the target directory and publish
/// via hard link (never overwrite). The owner removes the staging name.
/// This assumes no concurrent same-UID writer to the private directory.
pub fn publish_verified(staged: &Path, target: &Path, expected: ExpectedContent) -> io::Result<()> {
    validate_private_target(target)?;
    if staged.parent() != target.parent() {
        return Err(invalid());
    }
    let before = fs::symlink_metadata(staged)?;
    if !safe_file(&before, expected) {
        return Err(invalid());
    }
    let mut file = File::open(staged)?;
    let opened = file.metadata()?;
    if !safe_file(&opened, expected) || !unchanged(&before, &opened) {
        return Err(invalid());
    }
    let mut digest = Sha256::new();
    let mut remaining = expected.size;
    let mut buffer = [0u8; 64 * 1024];
    while remaining > 0 {
        let n = remaining.min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..n])?;
        digest.update(&buffer[..n]);
        remaining -= n as u64;
    }
    let actual: [u8; 32] = digest.finalize().into();
    if actual != expected.digest || !unchanged(&opened, &file.metadata()?)
        || !unchanged(&opened, &fs::symlink_metadata(staged)?)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "download integrity mismatch"));
    }
    file.sync_all()?;
    fs::hard_link(staged, target)
}
