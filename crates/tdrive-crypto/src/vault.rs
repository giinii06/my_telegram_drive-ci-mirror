//! Legacy vault XChaCha20-Poly1305 envelopes. Password derivation and
//! session key ownership are separate; callers must supply a derived KEK.
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand_core::{OsRng, RngCore};
use std::{error::Error, fmt};

const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const CHECK: &[u8] = b"tdrive-key-check-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultEnvelopeError {
    MalformedEnvelope,
    AuthenticationFailed,
    EntropyUnavailable,
}

impl fmt::Display for VaultEnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MalformedEnvelope => "invalid vault envelope",
            Self::AuthenticationFailed => "vault authentication failed",
            Self::EntropyUnavailable => "vault nonce generation failed",
        })
    }
}

impl Error for VaultEnvelopeError {}

fn cipher(key: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(key.into())
}

fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, VaultEnvelopeError> {
    let mut nonce = [0; NONCE_LEN];
    OsRng
        .try_fill_bytes(&mut nonce)
        .map_err(|_| VaultEnvelopeError::EntropyUnavailable)?;
    let ciphertext = cipher(key)
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| VaultEnvelopeError::AuthenticationFailed)?;
    let mut output = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn open(
    key: &[u8; 32],
    envelope: &[u8],
    plaintext_len: usize,
) -> Result<Vec<u8>, VaultEnvelopeError> {
    if envelope.len() != NONCE_LEN + plaintext_len + TAG_LEN {
        return Err(VaultEnvelopeError::MalformedEnvelope);
    }
    cipher(key)
        .decrypt(
            XNonce::from_slice(&envelope[..NONCE_LEN]),
            &envelope[NONCE_LEN..],
        )
        .map_err(|_| VaultEnvelopeError::AuthenticationFailed)
}

/// Go WrapMasterKey compatibility: nonce24 || encrypted key32 || tag16.
pub fn wrap_master_key(
    master: &[u8; 32],
    kek: &[u8; 32],
) -> Result<Vec<u8>, VaultEnvelopeError> {
    seal(kek, master)
}

pub fn unwrap_master_key(
    envelope: &[u8],
    kek: &[u8; 32],
) -> Result<[u8; 32], VaultEnvelopeError> {
    let mut plaintext = open(kek, envelope, 32)?;
    let result = plaintext
        .as_slice()
        .try_into()
        .map_err(|_| VaultEnvelopeError::MalformedEnvelope);
    plaintext.fill(0);
    result
}

pub fn encode_key_check(master: &[u8; 32]) -> Result<Vec<u8>, VaultEnvelopeError> {
    seal(master, CHECK)
}

pub fn verify_key_check(
    master: &[u8; 32],
    envelope: &[u8],
) -> Result<(), VaultEnvelopeError> {
    let mut plaintext = open(master, envelope, CHECK.len())?;
    let valid = plaintext == CHECK;
    plaintext.fill(0);
    if valid {
        Ok(())
    } else {
        Err(VaultEnvelopeError::MalformedEnvelope)
    }
}
