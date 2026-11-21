//! Go-compatible TDE1 64-KiB streaming writer.
//!
//! Random salt/prefix are generated with the OS CSPRNG. A test-only internal
//! function accepts fixed entropy so the produced bytes can be compared to
//! Go's EncryptStream with the same 36 random bytes.

use crate::{CHUNK_BYTES, HEADER_BYTES, MAX_PLAINTEXT_BYTES, Header, HeaderError};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use sha2::Sha256;
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};

const SALT_BYTES: usize = 16;
const PREFIX_BYTES: usize = 20;
const CHUNK_SIZE: usize = 65_536;
const FINAL_BIT: u32 = 1 << 31;

#[derive(Debug)]
pub enum StreamError {
    InvalidKey,
    InvalidSize,
    EntropyUnavailable,
    DerivationFailed,
    EncryptFailed,
    InvalidHeader(HeaderError),
    AuthenticationFailed,
    PlaintextSizeMismatch,
    Io(io::Error),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => f.write_str("TDE1 requires a 32-byte master key"),
            Self::InvalidSize => f.write_str("TDE1 plaintext length cannot be represented"),
            Self::EntropyUnavailable => f.write_str("OS secure randomness unavailable"),
            Self::DerivationFailed => f.write_str("TDE1 per-file HKDF derivation failed"),
            Self::EncryptFailed => f.write_str("TDE1 AEAD encryption failed"),
            Self::InvalidHeader(_) => f.write_str("TDE1 invalid stream header"),
            Self::AuthenticationFailed => f.write_str("TDE1 ciphertext authentication failed"),
            Self::PlaintextSizeMismatch => {
                f.write_str("TDE1 plaintext stream does not match declared byte length")
            }
            Self::Io(_) => f.write_str("TDE1 stream IO error"),
        }
    }
}

impl Error for StreamError {}

impl From<io::Error> for StreamError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Encrypts a known-size reader into TDE1 in constant-size chunks.
///
/// The returned writer can contain a partial ciphertext if a later read/write
/// fails. The caller MUST stage output and publish it only after Ok(()).
/// This API has no file deletion, no Telegram and no vault unlock behavior.
pub fn encrypt_stream<R: Read, W: Write>(
    source: &mut R,
    output: &mut W,
    master_key: &[u8],
    plaintext_size: i64,
) -> Result<(), StreamError> {
    if master_key.len() != 32 {
        return Err(StreamError::InvalidKey);
    }
    if !(0..=MAX_PLAINTEXT_BYTES).contains(&plaintext_size) {
        return Err(StreamError::InvalidSize);
    }
    let mut salt = [0u8; SALT_BYTES];
    let mut prefix = [0u8; PREFIX_BYTES];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut salt)
        .map_err(|_| StreamError::EntropyUnavailable)?;
    rng.try_fill_bytes(&mut prefix)
        .map_err(|_| StreamError::EntropyUnavailable)?;
    encrypt_with_material(source, output, master_key, plaintext_size, salt, prefix)
}

fn encrypt_with_material<R: Read, W: Write>(
    source: &mut R,
    output: &mut W,
    master_key: &[u8],
    plaintext_size: i64,
    salt: [u8; SALT_BYTES],
    prefix: [u8; PREFIX_BYTES],
) -> Result<(), StreamError> {
    if master_key.len() != 32 {
        return Err(StreamError::InvalidKey);
    }
    if !(0..=MAX_PLAINTEXT_BYTES).contains(&plaintext_size) {
        return Err(StreamError::InvalidSize);
    }
    let hk = Hkdf::<Sha256>::new(Some(&salt), master_key);
    let mut key = [0u8; 32];
    hk.expand(b"tdrive/file/v1", &mut key)
        .map_err(|_| StreamError::DerivationFailed)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key);
    key.fill(0);
    let cipher = cipher.map_err(|_| StreamError::DerivationFailed)?;

    let mut header = [0u8; HEADER_BYTES];
    header[0..4].copy_from_slice(b"TDE1");
    header[4] = 0;
    header[5] = 16;
    header[6..22].copy_from_slice(&salt);
    header[22..42].copy_from_slice(&prefix);
    header[42..50].copy_from_slice(&(plaintext_size as u64).to_le_bytes());
    output.write_all(&header)?;
    header.fill(0);

    let mut chunk = [0u8; CHUNK_SIZE];
    let mut remaining = plaintext_size as u64;
    let mut counter = 0u32;
    while remaining > 0 {
        let amount = remaining.min(CHUNK_BYTES as u64) as usize;
        match source.read_exact(&mut chunk[..amount]) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                chunk.fill(0);
                return Err(StreamError::PlaintextSizeMismatch);
            }
            Err(error) => {
                chunk.fill(0);
                return Err(StreamError::Io(error));
            }
        }
        remaining -= amount as u64;
        let is_final = remaining == 0 && amount < CHUNK_SIZE;
        if is_final {
            if let Err(error) = assert_no_trailing_plaintext(source) {
                chunk.fill(0);
                return Err(error);
            }
        }
        let result = seal_chunk(&cipher, output, &prefix, counter, is_final, &chunk[..amount]);
        chunk[..amount].fill(0);
        result?;
        counter += 1;
        if is_final {
            return Ok(());
        }
    }

    // Empty streams and exact multiples of 64 KiB always end with a
    // distinct authenticated empty final chunk, as in Go EncryptStream.
    assert_no_trailing_plaintext(source)?;
    seal_chunk(&cipher, output, &prefix, counter, true, &[])?;
    Ok(())
}


/// Bounded-memory TDE1 decryption. Each chunk is authenticated before writing.
/// The caller must use a private disposable staging file and publish only on
/// successful return; a late authentication error may leave earlier plaintext.
pub fn decrypt_stream<R: Read, W: Write>(
    source: &mut R,
    output: &mut W,
    master_key: &[u8],
) -> Result<i64, StreamError> {
    if master_key.len() != 32 {
        return Err(StreamError::InvalidKey);
    }
    let mut header_bytes = [0u8; HEADER_BYTES];
    source
        .read_exact(&mut header_bytes)
        .map_err(|error| match error.kind() {
            io::ErrorKind::UnexpectedEof => StreamError::InvalidHeader(HeaderError::TooShort),
            _ => StreamError::Io(error),
        })?;
    let header = Header::parse(&header_bytes).map_err(StreamError::InvalidHeader)?;
    header_bytes.fill(0);

    let hk = Hkdf::<Sha256>::new(Some(&header.salt), master_key);
    let mut key = [0u8; 32];
    hk.expand(b"tdrive/file/v1", &mut key)
        .map_err(|_| StreamError::DerivationFailed)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key);
    key.fill(0);
    let cipher = cipher.map_err(|_| StreamError::DerivationFailed)?;

    let mut ciphertext = [0u8; CHUNK_SIZE + 16];
    let mut index = 0u32;
    let mut total = 0i64;
    loop {
        let mut used = 0;
        while used < ciphertext.len() {
            match source.read(&mut ciphertext[used..]) {
                Ok(0) => break,
                Ok(n) => used += n,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    ciphertext.fill(0);
                    return Err(StreamError::Io(error));
                }
            }
        }
        if used < 16 {
            ciphertext.fill(0);
            return Err(StreamError::AuthenticationFailed);
        }
        let final_chunk = used < ciphertext.len();
        let mut nonce = [0u8; 24];
        nonce[..20].copy_from_slice(&header.nonce_prefix);
        let bits = if final_chunk { index | FINAL_BIT } else { index };
        nonce[20..].copy_from_slice(&bits.to_be_bytes());
        let decoded = cipher.decrypt(XNonce::from_slice(&nonce), &ciphertext[..used]);
        nonce.fill(0);
        ciphertext.fill(0);
        let mut plaintext = decoded.map_err(|_| StreamError::AuthenticationFailed)?;
        let new_total = total.checked_add(plaintext.len() as i64);
        if (!final_chunk && plaintext.len() != CHUNK_SIZE)
            || new_total.is_none()
            || new_total.unwrap_or(i64::MAX) > header.plaintext_size
        {
            plaintext.fill(0);
            return Err(StreamError::AuthenticationFailed);
        }
        total = new_total.ok_or(StreamError::AuthenticationFailed)?;
        let written = output.write_all(&plaintext);
        plaintext.fill(0);
        written.map_err(StreamError::Io)?;
        if final_chunk {
            if total != header.plaintext_size {
                return Err(StreamError::AuthenticationFailed);
            }
            return Ok(total);
        }
        index = index
            .checked_add(1)
            .ok_or(StreamError::AuthenticationFailed)?;
        if index & FINAL_BIT != 0 {
            return Err(StreamError::AuthenticationFailed);
        }
    }
}

fn seal_chunk<W: Write>(
    cipher: &XChaCha20Poly1305,
    output: &mut W,
    prefix: &[u8; PREFIX_BYTES],
    index: u32,
    final_chunk: bool,
    plaintext: &[u8],
) -> Result<(), StreamError> {
    let mut nonce = [0u8; 24];
    nonce[..20].copy_from_slice(prefix);
    let mut bits = index;
    if final_chunk {
        bits |= FINAL_BIT;
    }
    nonce[20..].copy_from_slice(&bits.to_be_bytes());
    let mut encrypted = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| StreamError::EncryptFailed)?;
    let result = output.write_all(&encrypted);
    encrypted.fill(0);
    nonce.fill(0);
    result.map_err(StreamError::Io)
}

fn assert_no_trailing_plaintext<R: Read>(source: &mut R) -> Result<(), StreamError> {
    let mut extra = [0u8; 1];
    for _ in 0..100 {
        match source.read(&mut extra) {
            Ok(0) => return Ok(()),
            Ok(_) => return Err(StreamError::PlaintextSizeMismatch),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(StreamError::Io(error)),
        }
    }
    Err(StreamError::Io(io::Error::other(
        "plaintext reader made no progress",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ciphertext_size, decrypt_small_tde1};
    use std::io::Cursor;

    fn pattern(size: usize) -> Vec<u8> {
        (0..size)
            .map(|index| ((index * 31 + 7) % 251) as u8)
            .collect()
    }

    #[test]
    fn go_tde1_chunk_boundaries_and_roundtrip() {
        let key = [0x42u8; 32];
        for size in [0, 1, 65_535, 65_536, 65_537, 196_625] {
            let plaintext = pattern(size);
            let mut output = Vec::new();
            encrypt_with_material(
                &mut Cursor::new(&plaintext),
                &mut output,
                &key,
                size as i64,
                [0x11; SALT_BYTES],
                [0x22; PREFIX_BYTES],
            )
            .unwrap();
            assert_eq!(output.len() as i64, ciphertext_size(size as i64));
            assert_eq!(decrypt_small_tde1(&output, &key).unwrap(), plaintext);
        }
    }

    #[test]
    fn refuses_bad_size_and_key_before_writing_any_ciphertext() {
        let mut output = Vec::new();
        assert!(matches!(
            encrypt_stream(&mut Cursor::new(b"hi"), &mut output, &[0u8; 31], 2),
            Err(StreamError::InvalidKey)
        ));
        assert!(output.is_empty());
        assert!(matches!(
            encrypt_stream(&mut Cursor::new(b"hi"), &mut output, &[0u8; 32], -1),
            Err(StreamError::InvalidSize)
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn truncated_or_longer_plaintext_is_rejected() {
        let key = [0x42u8; 32];
        for (plaintext, size) in [(b"short".as_slice(), 10_i64), (b"long".as_slice(), 2_i64)] {
            let mut output = Vec::new();
            assert!(matches!(
                encrypt_with_material(
                    &mut Cursor::new(plaintext),
                    &mut output,
                    &key,
                    size,
                    [0x11; SALT_BYTES],
                    [0x22; PREFIX_BYTES],
                ),
                Err(StreamError::PlaintextSizeMismatch)
            ));
        }
    }


    #[test]
    fn streaming_decrypt_crosses_go_chunk_boundaries() {
        let key = [0x42u8; 32];
        for size in [0, 1, 65_535, 65_536, 65_537, 196_625] {
            let expected = pattern(size);
            let mut encrypted = Vec::new();
            encrypt_with_material(
                &mut Cursor::new(&expected),
                &mut encrypted,
                &key,
                size as i64,
                [0x11; SALT_BYTES],
                [0x22; PREFIX_BYTES],
            )
            .unwrap();
            let mut plaintext = Vec::new();
            let actual =
                decrypt_stream(&mut Cursor::new(encrypted), &mut plaintext, &key).unwrap();
            assert_eq!(actual, size as i64);
            assert_eq!(plaintext, expected);
        }
    }

    #[test]
    fn streaming_decrypt_rejects_wrong_key_and_missing_final_tag() {
        let key = [0x42u8; 32];
        let mut encrypted = Vec::new();
        encrypt_with_material(
            &mut Cursor::new(b"secret"),
            &mut encrypted,
            &key,
            6,
            [0x11; SALT_BYTES],
            [0x22; PREFIX_BYTES],
        )
        .unwrap();
        let mut output = Vec::new();
        assert!(matches!(
            decrypt_stream(&mut Cursor::new(&encrypted), &mut output, &[0x33; 32]),
            Err(StreamError::AuthenticationFailed)
        ));
        assert!(output.is_empty());
        encrypted.pop();
        assert!(matches!(
            decrypt_stream(&mut Cursor::new(&encrypted), &mut output, &key),
            Err(StreamError::AuthenticationFailed)
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn streaming_decrypt_accepts_nine_megabyte_file() {
        let key = [0x42u8; 32];
        let size = 9 * 1024 * 1024 + 31;
        let expected = pattern(size);
        let mut ciphertext = Vec::new();
        encrypt_with_material(
            &mut Cursor::new(&expected),
            &mut ciphertext,
            &key,
            size as i64,
            [0x11; SALT_BYTES],
            [0x22; PREFIX_BYTES],
        )
        .unwrap();
        let mut actual = Vec::new();
        assert_eq!(
            decrypt_stream(&mut Cursor::new(ciphertext), &mut actual, &key).unwrap(),
            size as i64
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn seeded_go_encrypt_stream_bytes_match_rust() {
        let Ok(root) = std::env::var("TDRIVE_GO_TDE1_FIXTURES_DIR") else {
            eprintln!("Go crypto fixtures only present in GitLab/GitHub CI");
            return;
        };
        let key = [0x42u8; 32];
        for size in [0, 1, 65_535, 65_536, 65_537, 196_625] {
            let expected_path = std::path::Path::new(&root).join(format!("tde1-seeded-{size}.bin"));
            let expected = std::fs::read(expected_path).expect("Go seeded cipher fixture");
            let mut actual = Vec::new();
            encrypt_with_material(
                &mut Cursor::new(pattern(size)),
                &mut actual,
                &key,
                size as i64,
                [0x11; SALT_BYTES],
                [0x22; PREFIX_BYTES],
            )
            .unwrap();
            assert_eq!(actual, expected, "Go and Rust TDE1 bytes differ at {size}");
        }
    }
}
