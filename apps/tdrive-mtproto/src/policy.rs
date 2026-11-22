//! Fail-closed CLI parsing and filesystem guards, without network side effects.
use std::fmt;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    InvalidArguments,
    InvalidCredentials,
    UnsafePath,
}
impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidArguments => "invalid MTProto CLI arguments",
            Self::InvalidCredentials => "invalid Telegram developer API credentials",
            Self::UnsafePath => "unsafe local file or private session path",
        })
    }
}
impl std::error::Error for PolicyError {}

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Help,
    Status,
    Login,
    Dialogs,
    InboxStats,
    Ingest { peer: String, limit: usize },
    History { peer: String, limit: usize },
    Upload { peer: String, source: PathBuf },
    Download { peer: String, message_id: i32, destination: PathBuf },
}

fn valid_peer(value: &str) -> bool {
    if let Some(username) = value.strip_prefix('@') {
        (5..=32).contains(&username.len())
            && username.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    } else {
        (1..=64).contains(&value.len())
            && value.bytes().any(|c| c.is_ascii_digit())
            && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b':' || c == b'-' || c == b'_')
    }
}

fn absolute_simple(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|c| matches!(c, Component::CurDir | Component::ParentDir))
}

pub fn parse_action(args: &[String]) -> Result<CliAction, PolicyError> {
    match args {
        [] => Ok(CliAction::Help),
        [cmd] if cmd == "--help" => Ok(CliAction::Help),
        [cmd] if cmd == "status" => Ok(CliAction::Status),
        [cmd] if cmd == "login" => Ok(CliAction::Login),
        [cmd] if cmd == "dialogs" => Ok(CliAction::Dialogs),
        [cmd] if cmd == "inbox-stats" => Ok(CliAction::InboxStats),
        [cmd, peer, limit] if cmd == "ingest" && valid_peer(peer) => {
            let limit = limit.parse::<usize>().map_err(|_| PolicyError::InvalidArguments)?;
            if !(1..=100).contains(&limit) {
                return Err(PolicyError::InvalidArguments);
            }
            Ok(CliAction::Ingest { peer: peer.clone(), limit })
        }
        [cmd, peer, limit] if cmd == "history" && valid_peer(peer) => {
            let limit = limit.parse::<usize>().map_err(|_| PolicyError::InvalidArguments)?;
            if !(1..=100).contains(&limit) {
                return Err(PolicyError::InvalidArguments);
            }
            Ok(CliAction::History { peer: peer.clone(), limit })
        }
        [cmd, peer, file] if cmd == "upload" && valid_peer(peer) => {
            let source = PathBuf::from(file);
            if !absolute_simple(&source) {
                return Err(PolicyError::InvalidArguments);
            }
            Ok(CliAction::Upload { peer: peer.clone(), source })
        }
        [cmd, peer, message_id, file] if cmd == "download" && valid_peer(peer) => {
            let message_id = message_id.parse::<i32>().map_err(|_| PolicyError::InvalidArguments)?;
            let destination = PathBuf::from(file);
            if message_id <= 0 || !absolute_simple(&destination) {
                return Err(PolicyError::InvalidArguments);
            }
            Ok(CliAction::Download { peer: peer.clone(), message_id, destination })
        }
        _ => Err(PolicyError::InvalidArguments),
    }
}

pub fn validate_api_credentials(id: &str, hash: &str) -> Result<i32, PolicyError> {
    let id = id.parse::<i32>().map_err(|_| PolicyError::InvalidCredentials)?;
    if id <= 0 || hash.len() != 32 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(PolicyError::InvalidCredentials);
    }
    Ok(id)
}

pub fn validate_session_path(path: &Path) -> Result<(), PolicyError> {
    if !absolute_simple(path) {
        return Err(PolicyError::UnsafePath);
    }
    let parent = path.parent().ok_or(PolicyError::UnsafePath)?;
    let directory = fs::symlink_metadata(parent).map_err(|_| PolicyError::UnsafePath)?;
    if !directory.is_dir() || directory.file_type().is_symlink() || directory.permissions().mode() & 0o077 != 0 {
        return Err(PolicyError::UnsafePath);
    }
    match fs::symlink_metadata(path) {
        Ok(file) => {
            if !file.is_file() || file.file_type().is_symlink()
                || file.permissions().mode() & 0o077 != 0 || file.nlink() != 1 {
                Err(PolicyError::UnsafePath)
            } else {
                Ok(())
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(PolicyError::UnsafePath),
    }
}

pub fn validate_source(path: &Path) -> Result<(), PolicyError> {
    if !absolute_simple(path) {
        return Err(PolicyError::UnsafePath);
    }
    let entry = fs::symlink_metadata(path).map_err(|_| PolicyError::UnsafePath)?;
    if !entry.is_file() || entry.file_type().is_symlink() {
        return Err(PolicyError::UnsafePath);
    }
    Ok(())
}

pub fn validate_download_path(path: &Path) -> Result<(), PolicyError> {
    if !absolute_simple(path) {
        return Err(PolicyError::UnsafePath);
    }
    let parent = path.parent().ok_or(PolicyError::UnsafePath)?;
    let directory = fs::symlink_metadata(parent).map_err(|_| PolicyError::UnsafePath)?;
    if !directory.is_dir() || directory.file_type().is_symlink() {
        return Err(PolicyError::UnsafePath);
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(PolicyError::UnsafePath),
    }
}
