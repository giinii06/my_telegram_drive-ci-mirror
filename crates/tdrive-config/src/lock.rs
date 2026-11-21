//! Go-compatible backend.lock, with conservative no-reclaim semantics.
//!
//! The Go daemon uses exclusive creation of <config>/TDrive/backend.lock.
//! This crate also uses O_CREAT|O_EXCL and writes the exact Go Info JSON fields.
//! It must never remove a lock created by another process, even if its PID
//! appears stale: PID reuse and container namespaces make that unsafe.

use crate::{ConfigDirs, ConfigError};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockInfo {
    pub role: String,
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Debug)]
pub struct BackendLock {
    path: PathBuf,
    file: Option<File>,
    info: LockInfo,
}

impl BackendLock {
    /// Only one Go/Rust process can create this lock file at a time.
    /// Existing locks are never automatically reclaimed by Rust.
    pub fn acquire(dirs: &ConfigDirs, role: &str) -> Result<Self, ConfigError> {
        if role.is_empty()
            || role.len() > 48
            || !role
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(ConfigError::InvalidLockRole);
        }
        dirs.check_data_dir()?;
        #[cfg(not(unix))]
        {
            let _ = (dirs, role);
            return Err(ConfigError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let path = dirs.data_dir().join("backend.lock");
            let file = match OpenOptions::new()
                .write(true)
                .read(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(ConfigError::LockHeld);
                }
                Err(error) => return Err(error.into()),
            };
            let started_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ConfigError::UnsafePath)?
                .as_secs();
            let info = LockInfo {
                role: role.to_owned(),
                pid: std::process::id(),
                started_at,
            };
            let mut lock = Self {
                path,
                file: Some(file),
                info,
            };
            let data = format!(
                "{{\"role\":\"{}\",\"pid\":{},\"started_at\":{}}}\n",
                lock.info.role, lock.info.pid, lock.info.started_at
            );
            lock.file
                .as_mut()
                .ok_or(ConfigError::UnsafePath)?
                .write_all(data.as_bytes())?;
            lock.file
                .as_ref()
                .ok_or(ConfigError::UnsafePath)?
                .sync_all()?;
            File::open(dirs.data_dir())?.sync_all()?;
            Ok(lock)
        }
    }

    pub fn info(&self) -> &LockInfo {
        &self.info
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn release(mut self) -> Result<(), ConfigError> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<(), ConfigError> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        #[cfg(unix)]
        {
            let owned = file.metadata()?;
            drop(file);
            let current = match fs::symlink_metadata(&self.path) {
                Ok(current) => current,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.into()),
            };
            if !current.file_type().is_file()
                || current.dev() != owned.dev()
                || current.ino() != owned.ino()
            {
                return Err(ConfigError::LockReplaced);
            }
            fs::remove_file(&self.path)?;
            if let Some(parent) = self.path.parent() {
                File::open(parent)?.sync_all()?;
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            drop(file);
            Err(ConfigError::UnsupportedPlatform)
        }
    }
}

impl Drop for BackendLock {
    fn drop(&mut self) {
        let _ = self.release_inner();
    }
}
