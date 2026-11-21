//! Go's TDrive encrypted filename envelope (backend/crypto/name.go).
//!
//! v1: + unpadded URL-safe base64(nonce24 || XChaCha20Poly1305(name, aad="v1")).
//! Master key is never stored; a distinct HKDF salt domain-separates names
//! from TDE1 file chunk encryption keys. Only tests inject a fixed nonce.

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use sha2::Sha256;
use std::error::Error;
use std::fmt;

pub const NAME_KEY_VERSION: u32 = 1;
const VERSION: &str = "v1";
const MAX_NAME_BYTES: usize = 4096;
const NONCE_BYTES: usize = 24;
const TAG_BYTES: usize = 16;
const MAX_ENVELOPE_BYTES: usize = 3 + (NONCE_BYTES + MAX_NAME_BYTES + TAG_BYTES).div_ceil(3) * 4;
const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameCryptoError {
    InvalidKey,
    InvalidName,
    UnsupportedVersion,
    MalformedEnvelope,
    AuthenticationFailed,
    DerivationFailed,
    EntropyUnavailable,
}

impl fmt::Display for NameCryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidKey => "filename key must contain exactly 32 bytes",
            Self::InvalidName => "encrypted filename must be valid and 1..4096 UTF-8 bytes",
            Self::UnsupportedVersion => "unknown filename envelope version",
            Self::MalformedEnvelope => "invalid filename envelope encoding",
            Self::AuthenticationFailed => "filename authentication failed",
            Self::DerivationFailed => "filename HKDF key derivation failed",
            Self::EntropyUnavailable => "secure filename nonce generation failed",
        };
        f.write_str(message)
    }
}

impl Error for NameCryptoError {}

fn name_cipher(master_key: &[u8]) -> Result<XChaCha20Poly1305, NameCryptoError> {
    if master_key.len() != 32 {
        return Err(NameCryptoError::InvalidKey);
    }
    let hk = Hkdf::<Sha256>::new(Some(b"tdrive.name-seal.v1"), master_key);
    let mut key = [0_u8; 32];
    hk.expand(b"tdrive/file/v1", &mut key)
        .map_err(|_| NameCryptoError::DerivationFailed)?;
    let result = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| NameCryptoError::DerivationFailed);
    key.fill(0);
    result
}

/// Encrypt a UTF-8 filename with a fresh OS-generated XChaCha nonce.
///
/// The output exactly matches Go EncryptFileName for the same master key and
/// random nonce. No deterministic entropy can be requested from public code.
pub fn encrypt_file_name(master_key: &[u8], name: &str) -> Result<String, NameCryptoError> {
    validate_name(name)?;
    if master_key.len() != 32 {
        return Err(NameCryptoError::InvalidKey);
    }
    let mut nonce = [0_u8; NONCE_BYTES];
    OsRng
        .try_fill_bytes(&mut nonce)
        .map_err(|_| NameCryptoError::EntropyUnavailable)?;
    encrypt_with_nonce(master_key, name, &nonce)
}

fn encrypt_with_nonce(
    master_key: &[u8],
    name: &str,
    nonce: &[u8; NONCE_BYTES],
) -> Result<String, NameCryptoError> {
    validate_name(name)?;
    let cipher = name_cipher(master_key)?;
    let mut ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: name.as_bytes(),
                aad: VERSION.as_bytes(),
            },
        )
        .map_err(|_| NameCryptoError::AuthenticationFailed)?;
    let mut envelope = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
    envelope.extend_from_slice(nonce);
    envelope.extend_from_slice(&ciphertext);
    ciphertext.fill(0);
    let encoded = encode_url_raw(&envelope);
    envelope.fill(0);
    Ok(format!("{VERSION}:{encoded}"))
}

/// Decrypt and authenticate a Go/Rust v1 filename envelope.
///
/// Rejects wrong key, tampered tag/AAD/nonce, noncanonical Base64,
/// unsupported versions, invalid UTF-8, empty and oversized plaintext.
pub fn decrypt_file_name(master_key: &[u8], envelope: &str) -> Result<String, NameCryptoError> {
    if master_key.len() != 32 {
        return Err(NameCryptoError::InvalidKey);
    }
    let (version, body) = envelope
        .split_once(':')
        .ok_or(NameCryptoError::UnsupportedVersion)?;
    if version != VERSION {
        return Err(NameCryptoError::UnsupportedVersion);
    }
    if envelope.len() > MAX_ENVELOPE_BYTES {
        return Err(NameCryptoError::MalformedEnvelope);
    }
    let mut sealed = decode_url_raw(body)?;
    if sealed.len() < NONCE_BYTES + TAG_BYTES {
        sealed.fill(0);
        return Err(NameCryptoError::MalformedEnvelope);
    }

    let cipher = name_cipher(master_key)?;
    let opened = cipher.decrypt(
        XNonce::from_slice(&sealed[..NONCE_BYTES]),
        Payload {
            msg: &sealed[NONCE_BYTES..],
            aad: VERSION.as_bytes(),
        },
    );
    sealed.fill(0);
    let mut plaintext = opened.map_err(|_| NameCryptoError::AuthenticationFailed)?;
    let result = std::str::from_utf8(&plaintext)
        .map_err(|_| NameCryptoError::InvalidName)
        .and_then(|value| {
            validate_name(value)?;
            Ok(value.to_owned())
        });
    plaintext.fill(0);
    result
}

fn validate_name(name: &str) -> Result<(), NameCryptoError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(NameCryptoError::InvalidName);
    }
    Ok(())
}

fn encode_url_raw(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let a = chunk[0] as usize;
        let b = chunk[1] as usize;
        let c = chunk[2] as usize;
        out.push(B64_ALPHABET[a >> 2] as char);
        out.push(B64_ALPHABET[((a & 3) << 4) | (b >> 4)] as char);
        out.push(B64_ALPHABET[((b & 15) << 2) | (c >> 6)] as char);
        out.push(B64_ALPHABET[c & 63] as char);
    }
    let tail = chunks.remainder();
    if let Some(&a) = tail.first() {
        out.push(B64_ALPHABET[(a >> 2) as usize] as char);
        let second = ((a & 3) << 4) | (tail.get(1).copied().unwrap_or(0) >> 4);
        out.push(B64_ALPHABET[second as usize] as char);
        if let Some(&b) = tail.get(1) {
            out.push(B64_ALPHABET[((b & 15) << 2) as usize] as char);
        }
    }
    out
}

fn decode_url_raw(input: &str) -> Result<Vec<u8>, NameCryptoError> {
    if input.len() % 4 == 1 || input.len() > MAX_ENVELOPE_BYTES {
        return Err(NameCryptoError::MalformedEnvelope);
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3 + 3);
    let mut carry: u32 = 0;
    let mut bits: u32 = 0;
    for ch in input.bytes() {
        let value = match ch {
            b'A'..=b'Z' => (ch - b'A') as u32,
            b'a'..=b'z' => (ch - b'a' + 26) as u32,
            b'0'..=b'9' => (ch - b'0' + 52) as u32,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(NameCryptoError::MalformedEnvelope),
        };
        carry = (carry << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((carry >> bits) as u8);
            carry &= (1 << bits) - 1;
        }
    }
    if carry != 0 || encode_url_raw(&out) != input {
        out.fill(0);
        return Err(NameCryptoError::MalformedEnvelope);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_name(id: &str) -> String {
        match id {
            "ascii" => "movie.mkv".to_owned(),
            "korean" => "한글 파일 이름.mp4".to_owned(),
            "single" => "a".to_owned(),
            "emoji" => "사진🦋.jpg".to_owned(),
            "max" => "x".repeat(4096),
            _ => panic!("unexpected Go fixture ID"),
        }
    }

    #[test]
    fn exact_go_filename_envelope_byte_vectors() {
        let Ok(path) = std::env::var("TDRIVE_GO_NAME_FIXTURE") else {
            eprintln!("Go filename fixtures only present during parity CI");
            return;
        };
        let contents = std::fs::read_to_string(path).expect("Go name fixture");
        let mut seen = 0;
        for line in contents.lines() {
            let (id, expected) = line.split_once('\t').expect("fixture id and envelope");
            let actual =
                encrypt_with_nonce(&[11; 32], &go_name(id), &[0x19; 24]).unwrap();
            assert_eq!(actual, expected, "Go/Rust filename bytes differ: {id}");
            assert_eq!(
                decrypt_file_name(&[11; 32], &actual).unwrap(),
                go_name(id)
            );
            seen += 1;
        }
        assert_eq!(seen, 5, "Go must produce every committed fixture case");
    }

    #[test]
    fn strict_raw_url_base64_canonical_roundtrip() {
        for size in 0..=97 {
            let bytes = (0..size).map(|i| (i * 67 % 256) as u8).collect::<Vec<_>>();
            let encoded = encode_url_raw(&bytes);
            assert!(!encoded.contains('='));
            assert_eq!(decode_url_raw(&encoded).unwrap(), bytes);
        }
        // AB has the same decoded byte as AA, but its nonzero low 4 bits
        // would be noncanonical; Go's test fixtures are canonical.
        assert!(decode_url_raw("AB").is_err());
        for malformed in ["=", "A=", "A+", "A/", "A\nA", "A"] {
            assert!(decode_url_raw(malformed).is_err());
        }
    }

    #[test]
    fn rejects_ciphertexts_with_invalid_authenticated_utf8() {
        let key = [11; 32];
        let cipher = name_cipher(&key).unwrap();
        let nonce = [0x19; NONCE_BYTES];
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &[0xff, 0xfe],
                    aad: VERSION.as_bytes(),
                },
            )
            .unwrap();
        let mut sealed = nonce.to_vec();
        sealed.extend_from_slice(&ciphertext);
        let envelope = format!("v1:{}", encode_url_raw(&sealed));
        assert_eq!(
            decrypt_file_name(&key, &envelope),
            Err(NameCryptoError::InvalidName)
        );
    }
}
