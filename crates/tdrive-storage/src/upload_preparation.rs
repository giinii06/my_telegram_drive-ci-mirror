//! Offline upload preparation. Writers must represent disposable staging;
//! no remote sends or publication may occur inside the writer factory.
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

const MAX_PARTS: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadPart {
    pub index: u32,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPart {
    pub part: UploadPart,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadReceipt {
    pub size: u64,
    pub sha256: [u8; 32],
    pub parts: Vec<PreparedPart>,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid upload size or part count")
}

pub fn plan_upload_parts(size: u64, part_size: u64) -> io::Result<Vec<UploadPart>> {
    if size == 0 || size > i64::MAX as u64 || part_size == 0 || part_size > i64::MAX as u64 {
        return Err(invalid());
    }
    let count = size.div_ceil(part_size);
    if count > MAX_PARTS {
        return Err(invalid());
    }
    let mut parts = Vec::with_capacity(count as usize);
    let mut offset = 0;
    for index in 0..count {
        let length = part_size.min(size - offset);
        parts.push(UploadPart {
            index: index as u32,
            offset,
            size: length,
        });
        offset += length;
    }
    Ok(parts)
}

/// Split exact source bytes into bounded-memory staging writers, returning
/// per-part and overall SHA-256 only after every write and EOF check succeeds.
/// On failure all staging is invalid and the caller must discard it.
pub fn prepare_upload<R, W, F>(
    source: &mut R,
    size: u64,
    part_size: u64,
    mut open_part: F,
) -> io::Result<UploadReceipt>
where
    R: Read,
    W: Write,
    F: FnMut(UploadPart) -> io::Result<W>,
{
    let plan = plan_upload_parts(size, part_size)?;
    let mut parts = Vec::with_capacity(plan.len());
    let mut full = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    for part in plan {
        let mut writer = open_part(part)?;
        let mut hash = Sha256::new();
        let mut remaining = part.size;
        while remaining != 0 {
            let length = remaining.min(buffer.len() as u64) as usize;
            source.read_exact(&mut buffer[..length])?;
            writer.write_all(&buffer[..length])?;
            hash.update(&buffer[..length]);
            full.update(&buffer[..length]);
            remaining -= length as u64;
        }
        writer.flush()?;
        parts.push(PreparedPart { part, sha256: hash.finalize().into() });
    }
    let mut extra = [0u8; 1];
    loop {
        match source.read(&mut extra) {
            Ok(0) => break,
            Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "oversized upload source")),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(UploadReceipt { size, sha256: full.finalize().into(), parts })
}
