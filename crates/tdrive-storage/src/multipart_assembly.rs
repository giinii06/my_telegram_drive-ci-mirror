//! Assemble authenticated stored bytes into caller-owned disposable staging.
//! Failure may leave partial staging bytes; callers must discard them and
//! must never publish or delete original data based on an error result.
use crate::{FilePart, validate_multipart_parts};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

/// Fetch each validated part in index order. Read at most its declared size
/// plus one EOF probe, verify the aggregate SHA-256, and return the byte count.
/// A successful return is not atomic publication; that belongs to the caller.
pub fn assemble_multipart<R, W, F>(
    count: i64,
    size: i64,
    parts: &[FilePart],
    expected_sha256: [u8; 32],
    staging: &mut W,
    mut open_part: F,
) -> io::Result<u64>
where
    R: Read,
    W: Write,
    F: FnMut(&FilePart) -> io::Result<R>,
{
    if !(1..=100_000).contains(&count) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid part count"));
    }
    validate_multipart_parts(count, size, parts)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid multipart manifest"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    for part in parts {
        let mut reader = open_part(part)?;
        let mut remaining = part.size as u64;
        while remaining > 0 {
            let n = remaining.min(buffer.len() as u64) as usize;
            reader.read_exact(&mut buffer[..n])?;
            staging.write_all(&buffer[..n])?;
            digest.update(&buffer[..n]);
            remaining -= n as u64;
        }
        let mut extra = [0_u8; 1];
        loop {
            match reader.read(&mut extra) {
                Ok(0) => break,
                Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "oversized part")),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }
    let actual: [u8; 32] = digest.finalize().into();
    if actual != expected_sha256 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "multipart digest mismatch"));
    }
    staging.flush()?;
    Ok(size as u64)
}
