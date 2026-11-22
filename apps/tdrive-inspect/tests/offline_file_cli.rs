#![cfg(unix)]

// These tests invoke the actual diagnostic binary; a crypto crate unit test
// alone cannot prove CLI wiring or rejection semantics.
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct PrivateSandbox(PathBuf);

impl PrivateSandbox {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("monotonic wall clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tdrive-offline-cli-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        DirBuilder::new()
            .recursive(false)
            .mode(0o700)
            .create(&path)
            .expect("create private test directory");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn key(&self, name: &str, byte: u8) -> PathBuf {
        let path = self.path(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .expect("create private test key");
        file.write_all(&[byte; 32]).unwrap();
        path
    }
}

impl Drop for PrivateSandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command(action: &str, source: &Path, destination: &Path, key: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg(action)
        .arg(source)
        .arg(destination)
        .arg(key)
        .output()
        .expect("launch Rust diagnostic binary")
}

fn assert_no_temporary_files(sandbox: &PrivateSandbox) {
    for entry in fs::read_dir(&sandbox.0).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(
            !name.to_string_lossy().starts_with(".tdrive-crypto-"),
            "unpublished TDE1 temporary file leaked",
        );
    }
}

#[test]
fn offline_cli_roundtrips_empty_and_chunk_boundary_files() {
    let sandbox = PrivateSandbox::new();
    let key = sandbox.key("private.key", 0x42);
    for (index, length) in [0_usize, 1, 65_536, 65_537].into_iter().enumerate() {
        let plain = sandbox.path(&format!("plain-{index}"));
        let sealed = sandbox.path(&format!("sealed-{index}"));
        let restored = sandbox.path(&format!("restored-{index}"));
        let bytes: Vec<u8> = (0..length).map(|v| (v % 251) as u8).collect();
        fs::write(&plain, &bytes).unwrap();
        let encrypted = command("--encrypt-file", &plain, &sealed, &key);
        assert!(encrypted.status.success(), "encryption exit was not zero");
        let decrypted = command("--decrypt-file", &sealed, &restored, &key);
        assert!(decrypted.status.success(), "decryption exit was not zero");
        assert_eq!(fs::read(&restored).unwrap(), bytes);
        assert_eq!(fs::read(&plain).unwrap(), bytes);
        assert_eq!(fs::metadata(&sealed).unwrap().permissions().mode() & 0o077, 0);
        assert_no_temporary_files(&sandbox);
    }
}

#[test]
fn offline_cli_rejects_wrong_key_and_corrupted_last_tag_without_plaintext() {
    let sandbox = PrivateSandbox::new();
    let key = sandbox.key("private.key", 0x42);
    let wrong_key = sandbox.key("wrong.key", 0x55);
    let original = sandbox.path("plain");
    let encrypted = sandbox.path("sealed");
    let rejected = sandbox.path("should-not-exist");
    fs::write(&original, vec![0x44; 65_537]).unwrap();
    assert!(command("--encrypt-file", &original, &encrypted, &key).status.success());
    assert!(!command("--decrypt-file", &encrypted, &rejected, &wrong_key).status.success());
    assert!(!rejected.exists());
    let mut handle = OpenOptions::new().read(true).write(true).open(&encrypted).unwrap();
    handle.seek(SeekFrom::End(-1)).unwrap();
    handle.write_all(&[0xff]).unwrap();
    handle.sync_all().unwrap();
    assert!(!command("--decrypt-file", &encrypted, &rejected, &key).status.success());
    assert!(!rejected.exists());
    assert_no_temporary_files(&sandbox);
}

#[test]
fn offline_cli_never_replaces_existing_output() {
    let sandbox = PrivateSandbox::new();
    let key = sandbox.key("private.key", 0x42);
    let source = sandbox.path("plain");
    let existing = sandbox.path("existing");
    fs::write(&source, b"new input").unwrap();
    fs::write(&existing, b"do not replace").unwrap();
    assert!(!command("--encrypt-file", &source, &existing, &key).status.success());
    assert_eq!(fs::read(&existing).unwrap(), b"do not replace");
    assert_eq!(fs::read(&source).unwrap(), b"new input");
    assert_no_temporary_files(&sandbox);
}

#[test]
fn offline_cli_rejects_insecure_short_and_symlinked_keys() {
    let sandbox = PrivateSandbox::new();
    let key = sandbox.key("private.key", 0x42);
    let source = sandbox.path("plain");
    let target = sandbox.path("sealed");
    fs::write(&source, b"secret").unwrap();

    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!command("--encrypt-file", &source, &target, &key).status.success());
    assert!(!target.exists());

    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let short_key = sandbox.path("short.key");
    fs::write(&short_key, vec![0x42; 31]).unwrap();
    fs::set_permissions(&short_key, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(!command("--encrypt-file", &source, &target, &short_key).status.success());
    assert!(!target.exists());

    let link = sandbox.path("link.key");
    std::os::unix::fs::symlink(&key, &link).unwrap();
    assert!(!command("--encrypt-file", &source, &target, &link).status.success());
    assert!(!target.exists());
    assert_no_temporary_files(&sandbox);
}
