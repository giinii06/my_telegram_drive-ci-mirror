#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tdrive_crypto::{
    FileCryptoError, ciphertext_size, decrypt_file_atomic, encrypt_file_atomic,
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const TEST_KEY: [u8; 32] = [0x42; 32];

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let id = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tdrive-tde1-atomic-{}-{now}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn assert_no_staging(&self) {
        for entry in fs::read_dir(&self.0).unwrap() {
            let entry = entry.unwrap();
            let file_name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                !file_name.starts_with(".tdrive-crypto-"),
                "failed transaction leaked a temporary file: {file_name}"
            );
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn pattern(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| ((index * 31 + 7) % 251) as u8)
        .collect()
}

#[test]
fn file_roundtrip_beyond_old_eight_megabyte_limit() {
    let root = Scratch::new();
    let original = pattern(9 * 1024 * 1024 + 37);
    let plain = root.file("original.bin");
    let encrypted = root.file("encrypted.tde1");
    let restored = root.file("restored.bin");
    fs::write(&plain, &original).unwrap();

    assert_eq!(
        encrypt_file_atomic(&plain, &encrypted, &TEST_KEY).unwrap(),
        encrypted
    );
    assert_eq!(
        fs::metadata(&encrypted).unwrap().len(),
        ciphertext_size(original.len() as i64) as u64
    );
    assert_eq!(
        fs::metadata(&encrypted).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        decrypt_file_atomic(&encrypted, &restored, &TEST_KEY).unwrap(),
        restored
    );
    assert_eq!(fs::read(&restored).unwrap(), original);
    assert_eq!(fs::read(&plain).unwrap(), original);
    root.assert_no_staging();
}

#[test]
fn late_tag_corruption_leaves_no_published_plaintext_or_temp() {
    let root = Scratch::new();
    let plain = root.file("input.bin");
    let sealed = root.file("encrypted.tde1");
    let restored = root.file("restored.bin");
    fs::write(&plain, pattern(150_000)).unwrap();
    encrypt_file_atomic(&plain, &sealed, &TEST_KEY).unwrap();
    let mut ciphertext = fs::read(&sealed).unwrap();
    let last_byte = ciphertext.len() - 1;
    ciphertext[last_byte] ^= 0x80;
    fs::write(&sealed, &ciphertext).unwrap();

    let result = decrypt_file_atomic(&sealed, &restored, &TEST_KEY);
    assert!(matches!(result, Err(FileCryptoError::Crypto(_))));
    assert!(!restored.exists(), "failed authentication leaked published file");
    root.assert_no_staging();
}

#[test]
fn wrong_key_or_truncated_final_marker_does_not_publish() {
    let root = Scratch::new();
    let plain = root.file("plain.bin");
    let sealed = root.file("sealed.tde1");
    let output = root.file("published.bin");
    fs::write(&plain, pattern(65_536)).unwrap();
    encrypt_file_atomic(&plain, &sealed, &TEST_KEY).unwrap();

    assert!(decrypt_file_atomic(&sealed, &output, &[0x99; 32]).is_err());
    assert!(!output.exists());

    let bytes = fs::read(&sealed).unwrap();
    fs::write(&sealed, &bytes[..bytes.len() - 1]).unwrap();
    assert!(decrypt_file_atomic(&sealed, &output, &TEST_KEY).is_err());
    assert!(!output.exists());
    root.assert_no_staging();
}

#[test]
fn existing_file_and_symlink_are_never_replaced() {
    let root = Scratch::new();
    let plain = root.file("plain.bin");
    let encrypted = root.file("encrypted.tde1");
    let existing = root.file("keep.bin");
    let output_link = root.file("destination-link.bin");
    fs::write(&plain, b"sample input").unwrap();
    fs::write(&existing, b"operator-owned").unwrap();

    assert!(matches!(
        encrypt_file_atomic(&plain, &existing, &TEST_KEY),
        Err(FileCryptoError::AlreadyExists)
    ));
    assert_eq!(fs::read(&existing).unwrap(), b"operator-owned");
    encrypt_file_atomic(&plain, &encrypted, &TEST_KEY).unwrap();

    assert!(matches!(
        decrypt_file_atomic(&encrypted, &existing, &TEST_KEY),
        Err(FileCryptoError::AlreadyExists)
    ));
    symlink(&existing, &output_link).unwrap();
    assert!(matches!(
        decrypt_file_atomic(&encrypted, &output_link, &TEST_KEY),
        Err(FileCryptoError::AlreadyExists)
    ));
    assert_eq!(fs::read(&existing).unwrap(), b"operator-owned");
    root.assert_no_staging();
}

#[test]
fn unsafe_paths_and_public_directory_fail_closed() {
    let root = Scratch::new();
    let plain = root.file("plain.bin");
    let linked_source = root.file("source-link");
    let target = root.file("target.tde1");
    fs::write(&plain, b"unchanged").unwrap();
    symlink(&plain, &linked_source).unwrap();
    assert!(matches!(
        encrypt_file_atomic(&linked_source, &target, &TEST_KEY),
        Err(FileCryptoError::UnsafeSource)
    ));
    assert!(!target.exists());

    let other = root.file("other");
    fs::create_dir(&other).unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        encrypt_file_atomic(&plain, &other.join("bad.tde1"), &TEST_KEY),
        Err(FileCryptoError::UnsafeDestination)
    ));

    let linked_parent = root.file("linked");
    symlink(&other, &linked_parent).unwrap();
    assert!(matches!(
        encrypt_file_atomic(&plain, &linked_parent.join("bad.tde1"), &TEST_KEY),
        Err(FileCryptoError::UnsafeDestination)
    ));
    assert!(!other.join("bad.tde1").exists());
    assert_eq!(fs::read(plain).unwrap(), b"unchanged");
    root.assert_no_staging();
}

#[test]
fn rejects_relative_paths_and_wrong_key_without_touching_output() {
    let root = Scratch::new();
    let source = root.file("data.bin");
    let destination = root.file("out.tde1");
    fs::write(&source, b"preserve me").unwrap();
    assert!(matches!(
        encrypt_file_atomic(Path::new("data.bin"), &destination, &TEST_KEY),
        Err(FileCryptoError::UnsafeSource)
    ));
    assert!(matches!(
        encrypt_file_atomic(&source, &destination, &[0u8; 31]),
        Err(FileCryptoError::Crypto(_))
    ));
    assert!(!destination.exists());
    assert_eq!(fs::read(&source).unwrap(), b"preserve me");
    root.assert_no_staging();
}
