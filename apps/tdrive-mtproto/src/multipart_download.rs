//! Offline validation shared by the native multipart command and its tests.
use std::io;
use tdrive_storage::{FilePart, validate_multipart_parts};

pub use tdrive_storage::copy_exact_part;

pub fn validate_manifest(parts: &[FilePart], expected_size: u64) -> io::Result<()> {
    if parts.is_empty() || parts.len() > 100_000 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid part count"));
    }
    let size = i64::try_from(expected_size)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid stored size"))?;
    validate_multipart_parts(parts.len() as i64, size, parts)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid multipart manifest"))?;
    if parts.iter().any(|part| i32::try_from(part.msg_id).is_err()) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid Telegram message ID"));
    }
    Ok(())
}
