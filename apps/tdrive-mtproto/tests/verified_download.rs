#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_mtproto::policy::{CliAction, parse_action};
use tdrive_mtproto::verified_download::{ExpectedContent, publish_verified};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("tdrive-verified-{}-{now}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn expected(bytes: &[u8]) -> ExpectedContent {
    ExpectedContent::parse(&bytes.len().to_string(), &format!("{:x}", Sha256::digest(bytes))).unwrap()
}

#[test]
fn verified_download_command_is_explicit_and_validated() {
    let hash = "a".repeat(64);
    let args = |size: &str, digest: &str| {
        ["download-verified", "@owner", "42", "/tmp/out", size, digest]
            .iter().map(|s| s.to_string()).collect::<Vec<_>>()
    };
    assert!(matches!(parse_action(&args("3", &hash)), Ok(CliAction::DownloadVerified { message_id: 42, .. })));
    for (size, digest) in [("-1", hash.as_str()), ("3", "short"), ("+3", hash.as_str())] {
        assert!(parse_action(&args(size, digest)).is_err());
    }
}

#[test]
fn only_verified_bytes_are_published_without_clobbering() {
    let root = Scratch::new();
    let staged = root.0.join("staged");
    let output = root.0.join("output");
    fs::write(&staged, b"abc").unwrap();
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(publish_verified(&staged, &output, expected(b"xyz")).is_err());
    assert!(publish_verified(&staged, &output, expected(b"abcd")).is_err());
    assert!(!output.exists());
    fs::write(&output, b"existing").unwrap();
    assert!(publish_verified(&staged, &output, expected(b"abc")).is_err());
    assert_eq!(fs::read(&output).unwrap(), b"existing");
    fs::remove_file(&output).unwrap();
    publish_verified(&staged, &output, expected(b"abc")).unwrap();
    assert_eq!(fs::read(&output).unwrap(), b"abc");
}

#[test]
fn public_target_directory_is_rejected() {
    let root = Scratch::new();
    let staged = root.0.join("staged");
    let output = root.0.join("output");
    fs::write(&staged, b"abc").unwrap();
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(publish_verified(&staged, &output, expected(b"abc")).is_err());
    assert!(!output.exists());
}
