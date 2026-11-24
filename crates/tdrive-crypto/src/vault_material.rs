//! Allocation-sensitive Go vault metadata boundary. No KDF execution here.
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultKdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u8,
    pub key_len: u32,
    pub salt_len: usize,
}

impl Default for VaultKdfParams {
    fn default() -> Self {
        Self {
            memory_kib: 64 * 1024,
            iterations: 3,
            lanes: 4,
            key_len: 32,
            salt_len: 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultMaterialError {
    InvalidParams,
    UnsupportedKdf,
    InvalidEnvelope,
}

impl fmt::Display for VaultMaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidParams => "invalid vault KDF parameters",
            Self::UnsupportedKdf => "unsupported vault KDF",
            Self::InvalidEnvelope => "invalid vault envelope",
        })
    }
}

impl Error for VaultMaterialError {}

/// Legacy empty KDF represents Argon2id in Go's typed Params. JSON null or
/// an explicitly empty JSON string must be rejected by the eventual decoder.
/// This check never allocates a KDF workspace or decrypts untrusted input.
pub fn validate_vault_material(
    params: &VaultKdfParams,
    kdf: &str,
    salt: &[u8],
    wrapped: &[u8],
    check: &[u8],
) -> Result<(), VaultMaterialError> {
    if kdf != "" && kdf != "argon2id" {
        return Err(VaultMaterialError::UnsupportedKdf);
    }
    if !(16 * 1024..=256 * 1024).contains(&params.memory_kib)
        || !(1..=10).contains(&params.iterations)
        || !(1..=16).contains(&params.lanes)
        || params.key_len != 32
        || !(16..=64).contains(&params.salt_len)
        || salt.len() != params.salt_len
    {
        return Err(VaultMaterialError::InvalidParams);
    }
    if wrapped.len() != 72 || check.len() != 59 {
        return Err(VaultMaterialError::InvalidEnvelope);
    }
    Ok(())
}
