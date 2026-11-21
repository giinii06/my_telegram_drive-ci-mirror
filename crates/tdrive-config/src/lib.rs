//! TDrive-compatible private config and cache roots, plus atomic secret IO.
//!
//! Handles only the local filesystem; no live Telegram session, JSON schema
//! migration, or cross-platform ACL management is implied by this crate.
//! Refuses Windows secret writes until a Windows ACL implementation exists.

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

const APP_DIR: &str = "TDrive";
const MAX_SECRET_BYTES: usize = 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum ConfigError {
    InvalidRoot,
    UnsafePath,
    UnsafePermissions,
    UnsupportedPlatform,
    OversizedSecret,
    EmptySession,
    Io(std::io::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot => f.write_str("config base must be an absolute clean path"),
            Self::UnsafePath => f.write_str("symlink or non-directory in private config path"),
            Self::UnsafePermissions => f.write_str("private config path has unsafe permissions"),
            Self::UnsupportedPlatform => {
                f.write_str("secure Windows ACL support is not implemented")
            }
            Self::OversizedSecret => {
                f.write_str("config or session exceeded the 1 MiB safety limit")
            }
            Self::EmptySession => f.write_str("refusing to store an empty Telegram session"),
            Self::Io(e) => write!(f, "private config IO failure: {e}"),
        }
    }
}

impl Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SecretFile {
    Credentials,
    DriveSelection,
    Session,
    WebToken,
}

impl SecretFile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Credentials => "imp_config.json",
            Self::DriveSelection => "config.json",
            Self::Session => "session.json",
            Self::WebToken => "web.token",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigDirs {
    data: PathBuf,
    cache: PathBuf,
}

impl ConfigDirs {
    /// Explicit bases follow Go datadir.Set/SetCache semantics: both bases
    /// receive the final /TDrive component, and are independent.
    /// Directory creation is restricted to app-owned 0700 paths on Unix.
    pub fn from_bases(data_base: &Path, cache_base: &Path) -> Result<Self, ConfigError> {
        #[cfg(windows)]
        {
            let _ = (data_base, cache_base);
            return Err(ConfigError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let data = prepare_private_dir(&data_base.join(APP_DIR))?;
            let cache = prepare_private_dir(&cache_base.join(APP_DIR))?;
            Ok(Self { data, cache })
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = (data_base, cache_base);
            Err(ConfigError::UnsupportedPlatform)
        }
    }

    pub fn from_user_defaults() -> Result<Self, ConfigError> {
        Self::from_bases(&user_config_base()?, &user_cache_base()?)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    pub fn secret_path(&self, kind: SecretFile) -> PathBuf {
        self.data.join(kind.name())
    }

    /// Missing is not a corrupt session. An existing symlink or file with
    /// excessive permissions is a hard error and is never followed.
    pub fn read_private(&self, kind: SecretFile) -> Result<Option<Vec<u8>>, ConfigError> {
        self.check_data_dir()?;
        let path = self.secret_path(kind);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        check_private_file(&metadata)?;
        if metadata.len() > MAX_SECRET_BYTES as u64 {
            return Err(ConfigError::OversizedSecret);
        }
        let file = File::open(&path)?;
        let mut data = Vec::new();
        file.take((MAX_SECRET_BYTES + 1) as u64)
            .read_to_end(&mut data)?;
        if data.len() > MAX_SECRET_BYTES {
            return Err(ConfigError::OversizedSecret);
        }
        if kind == SecretFile::Session && data.is_empty() {
            return Err(ConfigError::EmptySession);
        }
        Ok(Some(data))
    }

    /// Safe, same-directory replacement with a private 0600 temporary file.
    /// Durable rename is followed by directory fsync on Unix. Existing files
    /// with non-private modes or symlink targets are rejected, not overwritten.
    pub fn write_private(&self, kind: SecretFile, bytes: &[u8]) -> Result<(), ConfigError> {
        self.check_data_dir()?;
        if bytes.len() > MAX_SECRET_BYTES {
            return Err(ConfigError::OversizedSecret);
        }
        if kind == SecretFile::Session && bytes.is_empty() {
            return Err(ConfigError::EmptySession);
        }

        let destination = self.secret_path(kind);
        match fs::symlink_metadata(&destination) {
            Ok(metadata) => check_private_file(&metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        #[cfg(not(unix))]
        {
            let _ = (destination, bytes);
            return Err(ConfigError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ConfigError::UnsafePath)?
                .as_nanos();
            let temp_path = self.data.join(format!(
                ".{}-{}-{nanos}-{seq}.tmp",
                kind.name(),
                std::process::id()
            ));
            let mut guard = UncommittedTemp(temp_path);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&guard.0)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            // The temp is inside our protected directory. Rename never
            // follows a destination symlink; it replaces the dir entry.
            fs::rename(&guard.0, &destination)?;
            guard.0 = PathBuf::new();
            File::open(&self.data)?.sync_all()?;
            Ok(())
        }
    }

    fn check_data_dir(&self) -> Result<(), ConfigError> {
        #[cfg(windows)]
        {
            return Err(ConfigError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            check_private_directory(&fs::symlink_metadata(&self.data)?)?;
            Ok(())
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(ConfigError::UnsupportedPlatform)
        }
    }
}

#[cfg(unix)]
struct UncommittedTemp(PathBuf);

#[cfg(unix)]
impl Drop for UncommittedTemp {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.0);
        }
    }
}

#[cfg(unix)]
fn check_private_directory(metadata: &fs::Metadata) -> Result<(), ConfigError> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ConfigError::UnsafePath);
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(ConfigError::UnsafePermissions);
    }
    Ok(())
}

#[cfg(unix)]
fn check_private_file(metadata: &fs::Metadata) -> Result<(), ConfigError> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ConfigError::UnsafePath);
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(ConfigError::UnsafePermissions);
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_private_file(_metadata: &fs::Metadata) -> Result<(), ConfigError> {
    Err(ConfigError::UnsupportedPlatform)
}

#[cfg(unix)]
fn prepare_private_dir(path: &Path) -> Result<PathBuf, ConfigError> {
    if !path.is_absolute() {
        return Err(ConfigError::InvalidRoot);
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::CurDir | Component::ParentDir) {
            return Err(ConfigError::InvalidRoot);
        }
        current.push(part.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(ConfigError::UnsafePath);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::DirBuilder::new().mode(0o700).create(&current)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    check_private_directory(&fs::symlink_metadata(path)?)?;
    Ok(path.to_path_buf())
}

/// Matches Go os.UserConfigDir path conventions for the supported hosts.
pub fn user_config_base() -> Result<PathBuf, ConfigError> {
    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?.join("Library/Application Support"));
    }
    #[cfg(target_os = "windows")]
    {
        return non_empty_env("APPDATA").ok_or(ConfigError::InvalidRoot);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = non_empty_env("XDG_CONFIG_HOME") {
            if xdg.is_absolute() {
                return Ok(xdg);
            }
        }
        Ok(home_dir()?.join(".config"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(ConfigError::UnsupportedPlatform)
    }
}

pub fn user_cache_base() -> Result<PathBuf, ConfigError> {
    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?.join("Library/Caches"));
    }
    #[cfg(target_os = "windows")]
    {
        return non_empty_env("LOCALAPPDATA").ok_or(ConfigError::InvalidRoot);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = non_empty_env("XDG_CACHE_HOME") {
            if xdg.is_absolute() {
                return Ok(xdg);
            }
        }
        Ok(home_dir()?.join(".cache"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(ConfigError::UnsupportedPlatform)
    }
}

#[cfg(unix)]
fn home_dir() -> Result<PathBuf, ConfigError> {
    non_empty_env("HOME").ok_or(ConfigError::InvalidRoot)
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| value != OsStr::new(""))
        .map(PathBuf::from)
}
