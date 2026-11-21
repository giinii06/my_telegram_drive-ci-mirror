//! Bounded, fully authenticated decoding of *small* Go TDE1 test ciphertext.
//!
//! This API does not stream partial plaintext, write files, unlock a vault,
//! or accept any user-defined encryption parameters. It returns plaintext only
//! after all XChaCha20-Poly1305 chunks and final-marker tag have authenticated.

use crate::{CHUNK_BYTES, HEADER_BYTES, Header, HeaderError, TAG_BYTES};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use sha2::Sha256;
use std::error::Error;
use std::fmt;

/// Bound memory while this is an offline differential-test facility. Streaming
/// decryption into securely staged temp files belongs to the later Phase 07.
pub const MAX_SMALL_PLAINTEXT: i64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecryptError {
    InvalidHeader(HeaderError),
    OversizedInput,
    InvalidKey,
    InvalidDerivation,
    AuthenticationFailed,
}

impl fmt::Display for DecryptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never put secret key, plaintext, ciphertext or credential in errors.
        match self {
            Self::InvalidHeader(e) => write!(f, "TDE1 header: {e}"),
            Self::OversizedInput => f.write_str("TDE1 input exceeds bounded decrypt limit"),
            Self::InvalidKey => f.write_str("TDE1 master key must have 32 bytes"),
            Self::InvalidDerivation => f.write_str("TDE1 key derivation failed"),
            Self::AuthenticationFailed => f.write_str("TDE1 authentication failed"),
        }
    }
}

impl Error for DecryptError {}

/// Decode a complete small TDE1 file produced by Go EncryptStream. The caller
/// receives no plaintext unless *all* AEAD tags, including the final chunk,
/// pass authentication. This is not a production streaming API.
pub fn decrypt_small_tde1(input: &[u8], master_key: &[u8]) -> Result<Vec<u8>, DecryptError> {
    if master_key.len() != 32 {
        return Err(DecryptError::InvalidKey);
    }
    let header = Header::parse(input).map_err(DecryptError::InvalidHeader)?;
    if header.plaintext_size > MAX_SMALL_PLAINTEXT {
        return Err(DecryptError::OversizedInput);
    }
    let stored = i64::try_from(input.len()).map_err(|_| DecryptError::OversizedInput)?;
    header
        .validate_stored_size(stored)
        .map_err(DecryptError::InvalidHeader)?;

    let hk = Hkdf::<Sha256>::new(Some(&header.salt), master_key);
    let mut subkey = [0_u8; 32];
    hk.expand(b"tdrive/file/v1", &mut subkey)
        .map_err(|_| DecryptError::InvalidDerivation)?;
    let cipher =
        XChaCha20Poly1305::new_from_slice(&subkey).map_err(|_| DecryptError::InvalidDerivation)?;
    subkey.fill(0);

    let size = usize::try_from(header.plaintext_size).map_err(|_| DecryptError::OversizedInput)?;
    let chunk_size = usize::try_from(CHUNK_BYTES).expect("fixed chunk size fits usize");
    let full_chunks = size / chunk_size;
    let final_size = size % chunk_size;
    let mut output = Vec::with_capacity(size);
    let mut offset = HEADER_BYTES;

    for counter in 0..=full_chunks {
        let last = counter == full_chunks;
        let chunk_len = if last { final_size } else { chunk_size };
        let stored_len = chunk_len + usize::try_from(TAG_BYTES).expect("fixed tag fits usize");
        let end = offset
            .checked_add(stored_len)
            .ok_or(DecryptError::AuthenticationFailed)?;
        let segment = input
            .get(offset..end)
            .ok_or(DecryptError::AuthenticationFailed)?;

        let mut nonce = [0_u8; 24];
        nonce[..20].copy_from_slice(&header.nonce_prefix);
        let mut counter_bits = u32::try_from(counter).map_err(|_| DecryptError::OversizedInput)?;
        if last {
            counter_bits |= 1 << 31;
        }
        nonce[20..].copy_from_slice(&counter_bits.to_be_bytes());
        let plaintext = cipher
            .decrypt(XNonce::from_slice(&nonce), segment)
            .map_err(|_| DecryptError::AuthenticationFailed)?;
        if plaintext.len() != chunk_len {
            return Err(DecryptError::AuthenticationFailed);
        }
        output.extend_from_slice(&plaintext);
        offset = end;
    }

    if offset != input.len() || output.len() != size {
        return Err(DecryptError::AuthenticationFailed);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_wrong_key_length_and_missing_header() {
        assert_eq!(
            decrypt_small_tde1(&[], &[0; 31]),
            Err(DecryptError::InvalidKey)
        );
        assert_eq!(
            decrypt_small_tde1(&[], &[0; 32]),
            Err(DecryptError::InvalidHeader(HeaderError::TooShort))
        );
    }

    #[test]
    fn refuses_oversized_plaintext_before_allocation() {
        let mut header = [0_u8; HEADER_BYTES];
        header[0..4].copy_from_slice(b"TDE1");
        header[5] = 16;
        header[42..50].copy_from_slice(&(MAX_SMALL_PLAINTEXT as u64 + 1).to_le_bytes());
        assert_eq!(
            decrypt_small_tde1(&header, &[0; 32]),
            Err(DecryptError::OversizedInput)
        );
    }
}
