//! Offline WebDAV protocol boundaries and exclusive lock registry.
//! No listener or live mutation handler is created here.
mod registry;
pub use registry::{LockError, LockRegistry};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockDepth {
    Zero,
    Infinity,
}

/// Go backend/mountdav/locks.go accepts exactly these values.
pub fn parse_lock_depth(value: &str) -> Option<LockDepth> {
    match value {
        "" | "infinity" => Some(LockDepth::Infinity),
        "0" => Some(LockDepth::Zero),
        _ => None,
    }
}

const DEFAULT_LOCK_SECONDS: u64 = 600;
const MAX_LOCK_SECONDS: u64 = 3600;

/// Select the first supported timeout token, capped at the legacy maximum.
pub fn parse_lock_timeout(value: &str) -> Option<Duration> {
    if value.is_empty() {
        return Some(Duration::from_secs(DEFAULT_LOCK_SECONDS));
    }
    for candidate in value.split(',').map(str::trim) {
        if candidate == "Infinite" {
            return Some(Duration::from_secs(MAX_LOCK_SECONDS));
        }
        let Some(raw) = candidate.strip_prefix("Second-") else {
            continue;
        };
        if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let Ok(seconds) = raw.parse::<u32>() else {
            continue;
        };
        if seconds > 0 {
            return Some(Duration::from_secs(u64::from(seconds).min(MAX_LOCK_SECONDS)));
        }
    }
    None
}
