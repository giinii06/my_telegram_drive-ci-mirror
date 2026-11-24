//! Local upload staging only. No Telegram session or send is opened.
//! The operator exclusively owns the destination parent.
use std::fmt::Write as _;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "unsafe upload preparation input")
}

fn absolute(path: &Path) -> bool {
    path.is_absolute() && !path.components().any(|part| matches!(part, Component::ParentDir))
}

struct Staging {
    path: PathBuf,
    committed: bool,
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    b.is_file() && a.dev() == b.dev() && a.ino() == b.ino()
        && a.len() == b.len() && a.mtime() == b.mtime()
        && a.mtime_nsec() == b.mtime_nsec() && a.ctime() == b.ctime()
        && a.ctime_nsec() == b.ctime_nsec()
}

fn hex(bytes: &[u8; 32]) -> String {
    let mut result = String::with_capacity(64);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("string write");
    }
    result
}

fn new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)
}

struct SyncedPart(File);
impl Write for SyncedPart {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }
}

pub fn run(source: &Path, destination: &Path, part_size: &str) -> io::Result<()> {
    if !absolute(source) || !absolute(destination) || part_size.is_empty()
        || !part_size.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }
    let part_size = part_size.parse::<u64>().map_err(|_| invalid())?;
    let parent = destination.parent().ok_or_else(invalid)?;
    let directory = fs::symlink_metadata(parent)?;
    if !directory.is_dir() || directory.permissions().mode() & 0o077 != 0 {
        return Err(invalid());
    }
    let before = fs::symlink_metadata(source)?;
    if !before.is_file() {
        return Err(invalid());
    }
    let mut input = File::open(source)?;
    if !same_file(&before, &input.metadata()?) {
        return Err(invalid());
    }
    tdrive_storage::plan_upload_parts(before.len(), part_size)?;
    DirBuilder::new().mode(0o700).create(destination)?;
    let mut stage = Staging { path: destination.to_path_buf(), committed: false };
    let receipt = tdrive_storage::prepare_upload(&mut input, before.len(), part_size, |part| {
        new_file(&stage.path.join(format!("{}.part", part.index))).map(SyncedPart)
    })?;
    if !same_file(&before, &input.metadata()?) || !same_file(&before, &fs::symlink_metadata(source)?) {
        return Err(invalid());
    }
    let mut json = format!("{{\"size\":\"{}\",\"sha256\":\"{}\",\"parts\":[", receipt.size, hex(&receipt.sha256));
    for (position, entry) in receipt.parts.iter().enumerate() {
        if position != 0 {
            json.push(',');
        }
        write!(&mut json,
            "{{\"index\":{},\"offset\":\"{}\",\"size\":\"{}\",\"sha256\":\"{}\"}}",
            entry.part.index, entry.part.offset, entry.part.size, hex(&entry.sha256)
        ).expect("string write");
    }
    json.push_str("]}\n");
    let mut manifest = new_file(&stage.path.join("receipt.json"))?;
    manifest.write_all(json.as_bytes())?;
    manifest.sync_all()?;
    File::open(&stage.path)?.sync_all()?;
    File::open(parent)?.sync_all()?;
    stage.committed = true;
    Ok(())
}
