//! One in-memory key owner per vault. Authentication and channel binding
//! remain the responsibility of the eventual service. No key export or
//! plaintext persistence, and no Clone/Debug implementation for the session.
use crate::{
    NameCryptoError, VaultEnvelopeError, decrypt_file_name, encrypt_file_name,
    unwrap_master_key, verify_key_check,
};
use std::{error::Error, fmt};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultSessionError {
    Locked,
    Envelope(VaultEnvelopeError),
    Name(NameCryptoError),
}

impl fmt::Display for VaultSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locked => f.write_str("vault is locked"),
            Self::Envelope(error) => error.fmt(f),
            Self::Name(error) => error.fmt(f),
        }
    }
}

impl Error for VaultSessionError {}

#[derive(Default)]
pub struct VaultSession {
    master: Option<Zeroizing<[u8; 32]>>,
}

impl VaultSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_unlocked(&self) -> bool {
        self.master.is_some()
    }

    /// Revoke the previous key before attempting unlock. The caller owns
    /// the derived KEK, its lifetime, password rate limits and vault binding.
    /// Failed verification never installs the candidate master key.
    pub fn unlock(
        &mut self,
        wrapped: &[u8],
        check: &[u8],
        kek: &[u8; 32],
    ) -> Result<(), VaultSessionError> {
        self.lock();
        if wrapped.len() != 72 || check.len() != 59 {
            return Err(VaultSessionError::Envelope(
                VaultEnvelopeError::MalformedEnvelope,
            ));
        }
        let candidate = Zeroizing::new(
            unwrap_master_key(wrapped, kek).map_err(VaultSessionError::Envelope)?,
        );
        verify_key_check(&candidate, check).map_err(VaultSessionError::Envelope)?;
        self.master = Some(candidate);
        Ok(())
    }

    /// Zeroizing clears the currently owned key on drop; it does not
    /// guarantee erasure of every compiler temporary or OS memory copy.
    pub fn lock(&mut self) {
        drop(self.master.take());
    }

    pub fn encrypt_name(&self, name: &str) -> Result<String, VaultSessionError> {
        let key = self.master.as_ref().ok_or(VaultSessionError::Locked)?;
        encrypt_file_name(&key[..], name).map_err(VaultSessionError::Name)
    }

    pub fn decrypt_name(&self, envelope: &str) -> Result<String, VaultSessionError> {
        let key = self.master.as_ref().ok_or(VaultSessionError::Locked)?;
        decrypt_file_name(&key[..], envelope).map_err(VaultSessionError::Name)
    }
}
