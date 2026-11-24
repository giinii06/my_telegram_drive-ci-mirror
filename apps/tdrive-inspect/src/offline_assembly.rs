//! Local recovery diagnostic. The operator exclusively owns both private
//! directories and supplies a trusted digest of the stored bytes.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid offline assembly input")
}

fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|component| {
            matches!(component, Component::ParentDir | Component::CurDir)
        })
}

fn private_dir(path: &Path) -> io::Result<PathBuf> {
    if !absolute(path) {
        return Err(invalid());
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(invalid());
    }
    fs::canonicalize(path)
}

fn positive(raw: &str) -> io::Result<i64> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    raw.parse::<i64>().ok().filter(|id| *id > 0).ok_or_else(invalid)
}

fn digest(raw: &str) -> io::Result<[u8; 32]> {
    if raw.len() != 64 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    let mut output = [0; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[index * 2..index * 2 + 2], 16)
            .map_err(|_| invalid())?;
    }
    Ok(output)
}

struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn open_part(directory: &Path, part: &tdrive_storage::FilePart) -> io::Result<File> {
    let path = directory.join(format!("{}.part", part.msg_id));
    let before = fs::symlink_metadata(&path)?;
    if !before.is_file() || before.nlink() != 1 || before.permissions().mode() & 0o077 != 0 {
        return Err(invalid());
    }
    let file = File::open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file()
        || opened.dev() != before.dev()
        || opened.ino() != before.ino()
        || opened.nlink() != 1
        || opened.permissions().mode() & 0o077 != 0
        || opened.len() != part.size as u64
    {
        return Err(invalid());
    }
    Ok(file)
}

/// Read only from the supplied detached snapshot and named local part files.
/// On any pre-publication error the stage is removed; the final link never
/// replaces a file. Crash durability of the directory entry is not claimed.
pub fn run(
    snapshot: &Path,
    channel: &str,
    message: &str,
    parts_dir: &Path,
    target: &Path,
    sha256: &str,
) -> io::Result<()> {
    if !absolute(snapshot) || !absolute(target) {
        return Err(invalid());
    }
    let expected = digest(sha256)?;
    let parts_dir = private_dir(parts_dir)?;
    let parent = private_dir(target.parent().ok_or_else(invalid)?)?;
    let target = parent.join(target.file_name().ok_or_else(invalid)?);
    match fs::symlink_metadata(&target) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Ok(_) => return Err(io::Error::new(io::ErrorKind::AlreadyExists, "target exists")),
        Err(error) => return Err(error),
    }
    let parts = tdrive_storage::resolve_multipart_snapshot(
        snapshot, positive(channel)?, positive(message)?,
    )
    .map_err(io::Error::other)?
    .ok_or_else(invalid)?;
    let size = parts.iter().try_fold(0_i64, |total, part| total.checked_add(part.size))
        .ok_or_else(invalid)?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?.as_nanos();
    let path = parent.join(format!(".tdrive-assembly-{}-{nonce}", std::process::id()));
    let mut output = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
    let stage = Stage(path);
    tdrive_storage::assemble_multipart(
        parts.len() as i64, size, &parts, expected, &mut output,
        |part| open_part(&parts_dir, part),
    )?;
    output.sync_all()?;
    drop(output);
    fs::hard_link(&stage.0, target)
}
