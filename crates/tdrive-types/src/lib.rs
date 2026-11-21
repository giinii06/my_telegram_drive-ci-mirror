//! Types shared by the gradual Rust migration.
//!
//! This crate intentionally has no GPUI, HTTP, SQLite, or Telegram dependency.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidId;

impl fmt::Display for InvalidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("identifiers must be positive")
    }
}

impl std::error::Error for InvalidId {}

impl ChannelId {
    pub fn new(value: i64) -> Result<Self, InvalidId> {
        if value <= 0 {
            return Err(InvalidId);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> i64 {
        self.0
    }

    /// JavaScript cannot precisely represent all Telegram 64-bit IDs as numbers.
    pub fn api_string(self) -> String {
        self.0.to_string()
    }
}

impl MessageId {
    pub fn new(value: i64) -> Result<Self, InvalidId> {
        if value <= 0 {
            return Err(InvalidId);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> i64 {
        self.0
    }

    pub fn api_string(self) -> String {
        self.0.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_require_positive_values() {
        for n in [0, -1, i64::MIN] {
            assert!(ChannelId::new(n).is_err());
            assert!(MessageId::new(n).is_err());
        }
        assert_eq!(MessageId::new(42).unwrap().get(), 42);
    }

    #[test]
    fn decimal_string_preserves_precision_past_javascript_limit() {
        let raw = 9_007_199_254_740_993_i64;
        assert_eq!(ChannelId::new(raw).unwrap().api_string(), "9007199254740993");
        assert_eq!(MessageId::new(i64::MAX).unwrap().api_string(), i64::MAX.to_string());
    }
}
