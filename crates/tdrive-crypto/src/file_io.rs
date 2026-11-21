//! Fail-closed on-disk boundary for Go-compatible TDE1 streams.
//!
//! This deliberately does not create directories or modify existing targets.
//! A caller must supply a private Unix directory and a new absolute filename.

use crate::{StreamError, ciphertext_size, decrypt_stream, encrypt_stream};
use rand_core::{OsRng, RngCore};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum FileCryptoError {
    UnsafeSource,
    UnsafeDestination,
    AlreadyExists,
    Crypto(StreamError),
    Io(io::Error),
}

impl fmt::Display for FileCryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeSource => f.write_str("unsafe TDE1 source path"),
            Self::UnsafeDestination => f.write_str("unsafe TDE1 destination directory"),
            Self::AlreadyExists => f.write_str("TDE1 destination already exists"),
            Self::Crypto(error) => write!(f, "TDE1 stream failed: {error}"),
            Self::Io(_) => f.write_str("TDE1 private-file IO failed"),
        }
    }
}

impl Error for FileCryptoError {}

impl From<io::Error> for FileCryptoError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

struct TemporaryOutput(PathBuf);

impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.0);
        }
    }
}

fn valid_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
}

// macOS maps the root-owned /var, /tmp and /etc directories through
// root-owned system symlinks into /private. Permit only these trusted aliases.
// All other symlinks, including user-created aliases, remain prohibited.
fn trusted_macos_system_alias(path: &Path, metadata: &fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        if path == Path::new("/var")
            || path == Path::new("/tmp")
            || path == Path::new("/etc")
        {
            return metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && fs::metadata(path)
                    .is_ok_and(|target| target.is_dir() && target.uid() == 0);
        }
    }
    let _ = (path, metadata);
    false
}

fn accepted_directory(path: &Path, metadata: &fs::Metadata) -> bool {
    (metadata.is_dir() && !metadata.file_type().is_symlink())
        || trusted_macos_system_alias(path, metadata)
}

fn check_parent_chain(parent: &Path) -> Result<(), FileCryptoError> {
    if !valid_absolute(parent) {
        return Err(FileCryptoError::UnsafeDestination);
    }
    for ancestor in parent.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        if !accepted_directory(ancestor, &metadata) {
            return Err(FileCryptoError::UnsafeDestination);
        }
    }
    let metadata = fs::symlink_metadata(parent)?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(FileCryptoError::UnsafeDestination);
    }
    Ok(())
}

fn open_checked_source(path: &Path) -> Result<File, FileCryptoError> {
    if !valid_absolute(path) {
        return Err(FileCryptoError::UnsafeSource);
    }
    // Refuse symlink ancestors. The source itself must be a regular file.
    let parent = path.parent().ok_or(FileCryptoError::UnsafeSource)?;
    for ancestor in parent.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        if !accepted_directory(ancestor, &metadata) {
            return Err(FileCryptoError::UnsafeSource);
        }
    }
    let before = fs::symlink_metadata(path)?;
    if !before.is_file() || before.file_type().is_symlink() {
        return Err(FileCryptoError::UnsafeSource);
    }
    let input = File::open(path)?;
    let opened = input.metadata()?;
    if !opened.is_file() || opened.dev() != before.dev() || opened.ino() != before.ino() {
        return Err(FileCryptoError::UnsafeSource);
    }
    Ok(input)
}

fn ensure_absent(path: &Path) -> Result<(), FileCryptoError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(FileCryptoError::AlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn random_temporary(parent: &Path) -> Result<(File, TemporaryOutput), FileCryptoError> {
    for _ in 0..8 {
        let mut random = [0u8; 16];
        OsRng
            .try_fill_bytes(&mut random)
            .map_err(|_| FileCryptoError::Crypto(StreamError::EntropyUnavailable))?;
        let id = u128::from_le_bytes(random);
        let path = parent.join(format!(".tdrive-crypto-{id:032x}.tmp"));
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&path);
        match file {
            Ok(file) => return Ok((file, TemporaryOutput(path))),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(FileCryptoError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many private staging file collisions",
    )))
}

fn stage_and_publish<F>(destination: &Path, operation: F) -> Result<PathBuf, FileCryptoError>
where
    F: FnOnce(&mut File) -> Result<(), FileCryptoError>,
{
    if !valid_absolute(destination) {
        return Err(FileCryptoError::UnsafeDestination);
    }
    let parent = destination
        .parent()
        .ok_or(FileCryptoError::UnsafeDestination)?;
    check_parent_chain(parent)?;
    ensure_absent(destination)?;

    let (mut file, mut temp) = random_temporary(parent)?;
    operation(&mut file)?;
    file.sync_all()?;
    drop(file);

    // On the same filesystem, hard_link provides atomic exclusive creation.
    // An existing path (even a symlink) is NEVER overwritten.
    fs::hard_link(&temp.0, destination)?;
    File::open(parent)?.sync_all()?;
    fs::remove_file(&temp.0)?;
    temp.0 = PathBuf::new();
    File::open(parent)?.sync_all()?;
    Ok(destination.to_path_buf())
}

/// Encrypt an existing regular file to a NEW private, durable TDE1 file.
///
/// The source is never modified; the destination is published only if the
/// complete stream and the final marker were successfully written. The parent
/// must already be private (0700) and must not be a symlink.
pub fn encrypt_file_atomic(
    source: &Path,
    destination: &Path,
    master_key: &[u8],
) -> Result<PathBuf, FileCryptoError> {
    let mut input = open_checked_source(source)?;
    let size = i64::try_from(input.metadata()?.len())
        .map_err(|_| FileCryptoError::Crypto(StreamError::InvalidSize))?;
    if ciphertext_size(size) == i64::MAX {
        return Err(FileCryptoError::Crypto(StreamError::InvalidSize));
    }
    stage_and_publish(destination, |temporary| {
        encrypt_stream(&mut input, temporary, master_key, size)
            .map_err(FileCryptoError::Crypto)
    })
}

/// Authenticate all ciphertext including the final marker before a decrypted
/// file becomes visible to any caller. Partial authenticated plaintext only
/// exists inside a private temporary file and is unlinked on failure.
pub fn decrypt_file_atomic(
    source: &Path,
    destination: &Path,
    master_key: &[u8],
) -> Result<PathBuf, FileCryptoError> {
    let mut input = open_checked_source(source)?;
    stage_and_publish(destination, |temporary| {
        decrypt_stream(&mut input, temporary, master_key)
            .map(|_| ())
            .map_err(FileCryptoError::Crypto)
    })
}
