//! Unix-only operator diagnostic for existing TDE1 file primitives.
//!
//! Not a vault, key derivation service, Telegram client or production upload.
//! CLI accepts a path to a strictly private 32-byte raw key FILE only, never
//! the key itself as a command line argument.
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path};

#[derive(Clone, Copy)]
pub enum Mode {
    Encrypt,
    Decrypt,
}

/// Key bytes should never be included in errors or formatted via Debug.
struct PrivateKey([u8; 32]);

impl Drop for PrivateKey {
    fn drop(&mut self) {
        // Best-effort wipe; this is not a substitute for a locked-memory vault.
        self.0.fill(0);
    }
}

fn read_private_key(path: &Path) -> Result<PrivateKey, &'static str> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err("private key path must be absolute");
    }
    let before = fs::symlink_metadata(path).map_err(|_| "private key unavailable")?;
    if !before.file_type().is_file()
        || before.nlink() != 1
        || before.permissions().mode() & 0o177 != 0
    {
        return Err("private key must be a regular, owner-only file");
    }

    let mut input = File::open(path).map_err(|_| "private key unavailable")?;
    let opened = input.metadata().map_err(|_| "private key unavailable")?;
    if !opened.is_file()
        || opened.dev() != before.dev()
        || opened.ino() != before.ino()
        || opened.nlink() != 1
        || opened.permissions().mode() & 0o177 != 0
        || opened.len() != 32
    {
        return Err("private key file changed or has invalid permissions/size");
    }

    let mut key = PrivateKey([0; 32]);
    input
        .read_exact(&mut key.0)
        .map_err(|_| "private key must contain exactly 32 raw bytes")?;
    let mut trailing = [0u8; 1];
    if input.read(&mut trailing).map_err(|_| "private key read failed")? != 0 {
        return Err("private key must contain exactly 32 raw bytes");
    }
    let after = input.metadata().map_err(|_| "private key unavailable")?;
    let named = fs::symlink_metadata(path).map_err(|_| "private key unavailable")?;
    for current in [&after, &named] {
        if !current.is_file()
            || current.dev() != opened.dev()
            || current.ino() != opened.ino()
            || current.nlink() != 1
            || current.permissions().mode() & 0o177 != 0
            || current.len() != 32
            || current.mtime() != opened.mtime()
            || current.mtime_nsec() != opened.mtime_nsec()
            || current.ctime() != opened.ctime()
            || current.ctime_nsec() != opened.ctime_nsec()
        {
            return Err("private key file changed during read");
        }
    }
    Ok(key)
}

/// Apply existing no-clobber, authenticated TDE1 file encryption/decryption.
/// No caller-controlled output is created until private key checks succeed.
pub fn run(
    mode: Mode,
    source: &Path,
    destination: &Path,
    key_file: &Path,
) -> Result<(), &'static str> {
    let key = read_private_key(key_file)?;
    let result = match mode {
        Mode::Encrypt => tdrive_crypto::encrypt_file_atomic(source, destination, &key.0),
        Mode::Decrypt => tdrive_crypto::decrypt_file_atomic(source, destination, &key.0),
    };
    result
        .map(|_| ())
        .map_err(|_| "source, destination or authenticated TDE1 operation was rejected")
}
