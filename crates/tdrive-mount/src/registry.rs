//! Bounded exclusive WebDAV lock state. An eventual handler must serialize
//! mutation confirmation and execution, authorize the caller, and normalize
//! Unicode to NFC before entering this module. A conflict query alone is
//! not an authorization or a mutation lease.
use crate::LockDepth;
use rand_core::{OsRng, RngCore};
use std::fmt::{self, Write as _};
use std::time::{Duration, Instant};

const MAX_ACTIVE: usize = 1024;
const MAX_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockError {
    InvalidPath,
    InvalidDuration,
    Conflict,
    Capacity,
    NotFound,
    EntropyUnavailable,
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidPath => "invalid canonical resource path",
            Self::InvalidDuration => "invalid lock duration",
            Self::Conflict => "resource is locked",
            Self::Capacity => "active lock limit reached",
            Self::NotFound => "lock not found for resource",
            Self::EntropyUnavailable => "lock token generation failed",
        })
    }
}

impl std::error::Error for LockError {}

struct Entry {
    root: String,
    token: String,
    depth: LockDepth,
    expires: Instant,
}

pub struct LockRegistry {
    maximum: usize,
    active: Vec<Entry>,
}

impl LockRegistry {
    pub fn new(maximum: usize) -> Self {
        Self {
            maximum: if maximum == 0 {
                MAX_ACTIVE
            } else {
                maximum.min(MAX_ACTIVE)
            },
            active: Vec::new(),
        }
    }

    fn expire(&mut self, now: Instant) {
        self.active.retain(|entry| entry.expires > now);
    }

    pub fn create(
        &mut self,
        now: Instant,
        root: &str,
        depth: LockDepth,
        ttl: Duration,
    ) -> Result<String, LockError> {
        validate_path(root)?;
        let expires = expiry(now, ttl)?;
        self.expire(now);
        if self.active.iter().any(|entry| {
            covers(&entry.root, entry.depth, root) || covers(root, depth, &entry.root)
        }) {
            return Err(LockError::Conflict);
        }
        if self.active.len() >= self.maximum {
            return Err(LockError::Capacity);
        }

        for _ in 0..4 {
            let token = random_token()?;
            if self.active.iter().any(|entry| entry.token == token) {
                continue;
            }
            self.active.push(Entry {
                root: root.to_owned(),
                token: token.clone(),
                depth,
                expires,
            });
            return Ok(token);
        }
        Err(LockError::EntropyUnavailable)
    }

    pub fn refresh(
        &mut self,
        now: Instant,
        root: &str,
        token: &str,
        ttl: Duration,
    ) -> Result<(), LockError> {
        validate_path(root)?;
        let expires = expiry(now, ttl)?;
        self.expire(now);
        let entry = self
            .active
            .iter_mut()
            .find(|entry| entry.root == root && entry.token == token)
            .ok_or(LockError::NotFound)?;
        entry.expires = expires;
        Ok(())
    }

    pub fn unlock(
        &mut self,
        now: Instant,
        root: &str,
        token: &str,
    ) -> Result<(), LockError> {
        validate_path(root)?;
        self.expire(now);
        let index = self
            .active
            .iter()
            .position(|entry| entry.root == root && entry.token == token)
            .ok_or(LockError::NotFound)?;
        self.active.swap_remove(index);
        Ok(())
    }

    pub(crate) fn token_applies(
        &mut self,
        now: Instant,
        path: &str,
        token: &str,
    ) -> Result<bool, LockError> {
        validate_path(path)?;
        self.expire(now);
        Ok(self.active.iter().any(|entry| {
            entry.token == token && covers(&entry.root, entry.depth, path)
        }))
    }

    /// Check all affected paths against one lock state, then run a synchronous
    /// mutation while the registry remains exclusively borrowed. An outer
    /// service must serialize this call with every other namespace mutation.
    pub fn with_mutation<T>(
        &mut self,
        now: Instant,
        targets: &[(&str, bool)],
        supplied_tokens: &[&str],
        operation: impl FnOnce() -> T,
    ) -> Result<T, LockError> {
        if targets.is_empty() || targets.len() > 2 {
            return Err(LockError::InvalidPath);
        }
        for (path, _) in targets {
            validate_path(path)?;
        }
        for &(path, recursive) in targets {
            if self.blocks_mutation(now, path, recursive, supplied_tokens)? {
                return Err(LockError::Conflict);
            }
        }
        Ok(operation())
    }

    /// Check all applicable locks, including descendants for recursive
    /// DELETE/MOVE. Supplied tokens must already have passed DAV If parsing.
    /// The caller must prevent other mutations/lock changes after this check.
    pub fn blocks_mutation(
        &mut self,
        now: Instant,
        path: &str,
        recursive: bool,
        supplied_tokens: &[&str],
    ) -> Result<bool, LockError> {
        validate_path(path)?;
        self.expire(now);
        Ok(self.active.iter().any(|entry| {
            let intersects = covers(&entry.root, entry.depth, path)
                || (recursive && covers(path, LockDepth::Infinity, &entry.root));
            intersects && !supplied_tokens.contains(&entry.token.as_str())
        }))
    }
}

fn expiry(now: Instant, ttl: Duration) -> Result<Instant, LockError> {
    if ttl.is_zero() || ttl > MAX_TTL {
        return Err(LockError::InvalidDuration);
    }
    now.checked_add(ttl).ok_or(LockError::InvalidDuration)
}

fn covers(root: &str, depth: LockDepth, target: &str) -> bool {
    root == target
        || (depth == LockDepth::Infinity
            && (root == "/"
                || target
                    .strip_prefix(root)
                    .is_some_and(|tail| tail.starts_with('/'))))
}

fn validate_path(path: &str) -> Result<(), LockError> {
    if !path.starts_with('/') || path.len() > 4096 || path.contains(['\\', '\0']) {
        return Err(LockError::InvalidPath);
    }
    if path != "/"
        && path[1..].split('/').any(|component| {
            component.is_empty()
                || component == "."
                || component == ".."
                || component.len() > 240
        })
    {
        return Err(LockError::InvalidPath);
    }
    Ok(())
}

fn random_token() -> Result<String, LockError> {
    let mut bytes = [0_u8; 16];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| LockError::EntropyUnavailable)?;
    let mut token = String::from("opaquelocktoken:");
    for byte in bytes {
        write!(&mut token, "{byte:02x}").expect("String writes cannot fail");
    }
    Ok(token)
}
