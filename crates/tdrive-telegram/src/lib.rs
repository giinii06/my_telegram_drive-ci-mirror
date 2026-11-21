//! Offline protocol contract for migrating gotd user-account MTProto to Rust.
//!
//! No network implementation is wired up. The existing Go backend remains
//! authoritative. In particular, Telegram Bot API is not a replacement for
//! the signed-in user MTProto protocol.

use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::num::NonZeroI64;
use std::pin::Pin;
use tdrive_types::{ChannelId, MessageId};

const RANDOM_ID_DOMAIN: &[u8] = b"tdrive.telegram.random-id.v1\0";
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    Empty,
    ExcessivelyLong,
    SurroundingWhitespace,
    ContainsNull,
    NonPositiveRandomId,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid durable Telegram idempotency key")
    }
}
impl Error for KeyError {}

fn validate_key(value: &str) -> Result<(), KeyError> {
    if value.is_empty() {
        return Err(KeyError::Empty);
    }
    if value.len() > MAX_IDEMPOTENCY_KEY_BYTES {
        return Err(KeyError::ExcessivelyLong);
    }
    if value.trim() != value {
        return Err(KeyError::SurroundingWhitespace);
    }
    if value.contains('\0') {
        return Err(KeyError::ContainsNull);
    }
    Ok(())
}

/// A positive Telegram MTProto random_id. Never synthesize a new one when a
/// write outcome is ambiguous; reuse the persisted operation id + step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RandomId(NonZeroI64);

impl RandomId {
    pub fn try_from_i64(raw: i64) -> Result<Self, KeyError> {
        if raw <= 0 {
            return Err(KeyError::NonPositiveRandomId);
        }
        Ok(Self(NonZeroI64::new(raw).expect("positive i64 is nonzero")))
    }

    pub const fn get(self) -> i64 {
        self.0.get()
    }
}

/// Byte-for-byte equivalent to backend/tgclient/idempotency.go StableRandomID:
/// SHA-256(domain | operation_id | NUL | step), high 8 bytes BE, mask sign.
pub fn stable_random_id(operation_id: &str, step: &str) -> Result<RandomId, KeyError> {
    validate_key(operation_id)?;
    validate_key(step)?;

    let mut sha = Sha256::new();
    sha.update(RANDOM_ID_DOMAIN);
    sha.update(operation_id.as_bytes());
    sha.update([0]);
    sha.update(step.as_bytes());

    let digest = sha.finalize();
    let mut first = [0_u8; 8];
    first.copy_from_slice(&digest[..8]);
    let positive = (u64::from_be_bytes(first) & i64::MAX as u64) as i64;
    RandomId::try_from_i64(positive.max(1))
}

/// The caller must persist both strings before first remote send.
#[derive(Clone, PartialEq, Eq)]
pub struct DurableSendKey {
    operation_id: String,
    step: String,
    random_id: RandomId,
}

impl DurableSendKey {
    pub fn new(operation_id: String, step: String) -> Result<Self, KeyError> {
        let random_id = stable_random_id(&operation_id, &step)?;
        Ok(Self {
            operation_id,
            step,
            random_id,
        })
    }

    pub fn random_id(&self) -> RandomId {
        self.random_id
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn step(&self) -> &str {
        &self.step
    }
}

/// Access hash is intentionally not Debug-printed, serialized to frontend,
/// or persisted as a JavaScript number.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InputPeer {
    pub channel_id: ChannelId,
    pub access_hash: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryCursor {
    pub min_id: i64,
    pub offset_id: i64,
    pub limit: u16,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HistoryMessage {
    pub msg_id: MessageId,
    pub timestamp: i64,
    pub actor_id: i64,
    pub text: String,
    pub has_media: bool,
    pub media_size: i64,
    pub placeholder: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ChannelDifference {
    pub pts: i64,
    pub final_page: bool,
    pub too_long: bool,
    pub new_messages: Vec<HistoryMessage>,
    pub deleted_ids: Vec<MessageId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelegramError {
    Unauthorized,
    NotFound,
    FloodWait { seconds: u32 },
    TransientTransport,
    /// The send may have succeeded; MUST retry with the same RandomId.
    SendOutcomeUnknown,
    InvalidRequest,
    Unsupported,
}

impl fmt::Display for TelegramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unauthorized => "Telegram authorization required",
            Self::NotFound => "Telegram message or channel not found",
            Self::FloodWait { .. } => "Telegram flood wait",
            Self::TransientTransport => "Telegram transport unavailable",
            Self::SendOutcomeUnknown => "Telegram send outcome unknown",
            Self::InvalidRequest => "invalid Telegram request",
            Self::Unsupported => "Telegram operation unsupported",
        })
    }
}
impl Error for TelegramError {}

pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, TelegramError>> + Send + 'a>>;

/// Object-safe async ports keep connection ownership inside the eventual
/// MTProto adapter, rather than letting UI/server code share raw TL types.
pub trait TelegramReadPort: Send + Sync {
    fn self_id(&self) -> PortFuture<'_, i64>;
    fn get_history(&self, peer: InputPeer, cursor: HistoryCursor)
        -> PortFuture<'_, Vec<HistoryMessage>>;
    fn get_channel_difference(
        &self, peer: InputPeer, pts: i64, limit: u16
    ) -> PortFuture<'_, ChannelDifference>;
    fn get_channel_pts(&self, peer: InputPeer) -> PortFuture<'_, i64>;
}

/// Fail-closed on uncertain writes. Only calls with persisted RandomId are
/// supported by this boundary, unlike old convenience SendControl methods.
pub trait TelegramWritePort: Send + Sync {
    fn send_control<'a>(
        &'a self,
        peer: InputPeer,
        caption: &'a str,
        silent: bool,
        random_id: RandomId,
    ) -> PortFuture<'a, MessageId>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStage {
    Unconfigured,
    AwaitPhone,
    AwaitCode,
    AwaitPassword,
    Authorized,
    RateLimited { seconds: u32 },
    Expired,
}

/// Bounded flood-wait retry budget from Go defaultWriteFloodWait{Retries,Max,Total}.
#[derive(Debug, Clone, Copy)]
pub struct FloodWaitBudget {
    pub retries_remaining: u8,
    pub per_wait_max_seconds: u32,
    pub total_wait_remaining_seconds: u32,
}

impl Default for FloodWaitBudget {
    fn default() -> Self {
        Self {
            retries_remaining: 2,
            per_wait_max_seconds: 30,
            total_wait_remaining_seconds: 60,
        }
    }
}

impl FloodWaitBudget {
    pub fn permit(&mut self, error: TelegramError) -> Option<u32> {
        let TelegramError::FloodWait { seconds } = error else {
            return None;
        };
        if self.retries_remaining == 0
            || seconds > self.per_wait_max_seconds
            || seconds > self.total_wait_remaining_seconds
        {
            return None;
        }
        self.retries_remaining -= 1;
        self.total_wait_remaining_seconds -= seconds;
        Some(seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_reuses_exact_same_random_id() {
        let first = DurableSendKey::new("upload-1".into(), "part:0".into()).unwrap();
        let again = DurableSendKey::new("upload-1".into(), "part:0".into()).unwrap();
        assert_eq!(first.random_id(), again.random_id());
        assert_ne!(
            first.random_id(),
            stable_random_id("upload-1", "part:1").unwrap()
        );
    }

    #[test]
    fn key_validation_rejects_whitespace_null_oversized_and_nonpositive() {
        for invalid in ["", " op", "op ", "op\n", "op\0id"] {
            assert!(stable_random_id(invalid, "step").is_err());
        }
        assert!(stable_random_id("op", &"x".repeat(257)).is_err());
        assert!(RandomId::try_from_i64(0).is_err());
        assert!(RandomId::try_from_i64(-1).is_err());
    }

    #[test]
    fn flood_wait_cannot_exceed_go_budgets() {
        let mut budget = FloodWaitBudget::default();
        assert_eq!(budget.permit(TelegramError::FloodWait { seconds: 31 }), None);
        assert_eq!(budget.permit(TelegramError::FloodWait { seconds: 30 }), Some(30));
        assert_eq!(budget.permit(TelegramError::FloodWait { seconds: 30 }), Some(30));
        assert_eq!(budget.permit(TelegramError::FloodWait { seconds: 1 }), None);
    }

    #[test]
    fn no_credentials_or_peer_hash_in_errors() {
        assert!(!TelegramError::SendOutcomeUnknown.to_string().contains("hash"));
        assert!(!TelegramError::Unauthorized.to_string().contains("token"));
    }
}
