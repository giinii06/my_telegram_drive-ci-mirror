//! Read an explicitly selected local media file, never a Telegram path.
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path};

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid local media source or range")
}

pub fn read_range(source: &Path, range: &str, output: &mut impl Write) -> io::Result<()> {
    if !source.is_absolute()
        || source.components().any(|part| matches!(part, Component::ParentDir))
        || range.len() > 1024
    {
        return Err(invalid());
    }
    let before = fs::symlink_metadata(source)?;
    if !before.is_file() {
        return Err(invalid());
    }
    let mut file = File::open(source)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() != before.len() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != opened.dev() || before.ino() != opened.ino() {
            return Err(invalid());
        }
    }
    let size = i64::try_from(opened.len()).map_err(|_| invalid())?;
    let decision = tdrive_media::copy_range(&mut file, output, range, size)?;
    if !decision.ok {
        return Err(invalid());
    }
    output.flush()
}
