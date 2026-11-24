//! TDE1 header inspection, small-file authentication and streaming encryption from Go.
//!
//! A valid header alone does not authenticate ciphertext. Streaming encrypt
//! is compatible with the original Go envelope, but callers must stage output
//! atomically and must not publish failed streams. Vault integration is pending.

mod auth;
#[cfg(unix)]
mod file_io;
mod name;
mod stream;
mod vault;
mod vault_session;
pub use vault_session::{VaultSession, VaultSessionError};
pub use vault::{
    VaultEnvelopeError, encode_key_check, unwrap_master_key, verify_key_check, wrap_master_key,
};
pub use name::{NAME_KEY_VERSION, NameCryptoError, decrypt_file_name, encrypt_file_name};
#[cfg(unix)]
pub use file_io::{FileCryptoError, decrypt_file_atomic, encrypt_file_atomic};
pub use auth::{DecryptError, MAX_SMALL_PLAINTEXT, decrypt_small_tde1};
pub use stream::{StreamError, decrypt_stream, encrypt_stream};

use std::error::Error;
use std::fmt;

pub const HEADER_BYTES: usize = 50;
pub const CHUNK_BYTES: i64 = 65_536;
pub const TAG_BYTES: i64 = 16;
pub const MAX_PLAINTEXT_BYTES: i64 = (1_i64 << 31) * CHUNK_BYTES - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    TooShort,
    BadMagic,
    Unsupported,
    CiphertextSizeMismatch,
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooShort => "TDE1 header is truncated",
            Self::BadMagic => "file is not a TDE1 stream",
            Self::Unsupported => "unsupported TDE1 envelope",
            Self::CiphertextSizeMismatch => "TDE1 stored size mismatch",
        };
        f.write_str(message)
    }
}

impl Error for HeaderError {}

#[derive(Clone, PartialEq, Eq)]
pub struct Header {
    pub plaintext_size: i64,
    pub salt: [u8; 16],
    pub nonce_prefix: [u8; 20],
}

impl Header {
    pub fn parse(input: &[u8]) -> Result<Self, HeaderError> {
        if input.len() < HEADER_BYTES {
            return Err(HeaderError::TooShort);
        }
        if &input[..4] != b"TDE1" {
            return Err(HeaderError::BadMagic);
        }
        if input[4] != 0 || input[5] != 16 {
            return Err(HeaderError::Unsupported);
        }
        let declared = u64::from_le_bytes(input[42..50].try_into().unwrap());
        let plaintext_size = i64::try_from(declared).map_err(|_| HeaderError::Unsupported)?;
        if plaintext_size > MAX_PLAINTEXT_BYTES {
            return Err(HeaderError::Unsupported);
        }
        Ok(Self {
            plaintext_size,
            salt: input[6..22].try_into().unwrap(),
            nonce_prefix: input[22..42].try_into().unwrap(),
        })
    }

    /// The size check only detects truncation/mismatch, not AEAD authentication.
    pub fn validate_stored_size(&self, stored_size: i64) -> Result<(), HeaderError> {
        if ciphertext_size(self.plaintext_size) != stored_size {
            return Err(HeaderError::CiphertextSizeMismatch);
        }
        Ok(())
    }
}

/// Exact Go CiphertextSize formula, including a final tag for a multiple of
/// the 64KiB block size and for an empty file.
pub fn ciphertext_size(plaintext_size: i64) -> i64 {
    if plaintext_size < 0 {
        return 0;
    }
    if plaintext_size > MAX_PLAINTEXT_BYTES {
        return i64::MAX;
    }
    let chunks = plaintext_size / CHUNK_BYTES + 1;
    (HEADER_BYTES as i64)
        .checked_add(plaintext_size)
        .and_then(|base| base.checked_add(chunks * TAG_BYTES))
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(size: u64) -> [u8; HEADER_BYTES] {
        let mut raw = [0_u8; HEADER_BYTES];
        raw[..4].copy_from_slice(b"TDE1");
        raw[5] = 16;
        raw[42..50].copy_from_slice(&size.to_le_bytes());
        raw
    }

    #[test]
    fn ciphertext_size_matches_go_at_all_final_tag_boundaries() {
        assert_eq!(ciphertext_size(-1), 0);
        assert_eq!(ciphertext_size(0), 66);
        assert_eq!(ciphertext_size(1), 67);
        assert_eq!(ciphertext_size(65_535), 65_601);
        assert_eq!(ciphertext_size(65_536), 65_618);
        assert_eq!(ciphertext_size(65_537), 65_619);
        assert_eq!(ciphertext_size(131_072), 131_170);
        assert_eq!(ciphertext_size(MAX_PLAINTEXT_BYTES + 1), i64::MAX);
    }

    #[test]
    fn header_matches_legacy_byte_offsets_and_little_endian_size() {
        let mut raw = make_header(65_536);
        raw[6..22].copy_from_slice(&[0x12; 16]);
        raw[22..42].copy_from_slice(&[0x34; 20]);
        let header = Header::parse(&raw).unwrap();
        assert_eq!(header.plaintext_size, 65_536);
        assert_eq!(header.salt, [0x12; 16]);
        assert_eq!(header.nonce_prefix, [0x34; 20]);
        assert!(header.validate_stored_size(65_618).is_ok());
        assert_eq!(
            header.validate_stored_size(65_617),
            Err(HeaderError::CiphertextSizeMismatch)
        );
    }

    #[test]
    fn malformed_header_is_rejected() {
        assert!(matches!(
            Header::parse(&make_header(0)[..49]),
            Err(HeaderError::TooShort)
        ));
        let mut bad = make_header(0);
        bad[0] = b'X';
        assert!(matches!(Header::parse(&bad), Err(HeaderError::BadMagic)));
        let mut bad = make_header(0);
        bad[4] = 1;
        assert!(matches!(Header::parse(&bad), Err(HeaderError::Unsupported)));
        let mut bad = make_header(0);
        bad[5] = 15;
        assert!(matches!(Header::parse(&bad), Err(HeaderError::Unsupported)));
        assert!(matches!(
            Header::parse(&make_header(u64::MAX)),
            Err(HeaderError::Unsupported)
        ));
    }
}
