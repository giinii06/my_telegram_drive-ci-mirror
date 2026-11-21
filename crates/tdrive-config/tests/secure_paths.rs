#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tdrive_config::{ConfigDirs, ConfigError, SecretFile};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "tdrive-config-test-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&base).unwrap();
        Self(base)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
#[test]
fn go_base_override_paths_and_private_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new();
    let data = scratch.path().join("data");
    let cache = scratch.path().join("cache");
    let paths = ConfigDirs::from_bases(&data, &cache).unwrap();
    assert_eq!(paths.data_dir(), data.join("TDrive"));
    assert_eq!(paths.cache_dir(), cache.join("TDrive"));
    assert_eq!(
        paths.secret_path(SecretFile::Credentials),
        data.join("TDrive/imp_config.json")
    );
    assert_eq!(
        paths.secret_path(SecretFile::DriveSelection),
        data.join("TDrive/config.json")
    );
    assert_eq!(
        paths.secret_path(SecretFile::Session),
        data.join("TDrive/session.json")
    );
    assert_eq!(
        paths.secret_path(SecretFile::WebToken),
        data.join("TDrive/web.token")
    );
    assert_eq!(
        fs::metadata(paths.data_dir()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(paths.cache_dir())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[cfg(unix)]
#[test]
fn atomic_session_replace_and_missing_are_distinct() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new();
    let paths = ConfigDirs::from_bases(scratch.path(), scratch.path()).unwrap();
    assert!(paths.read_private(SecretFile::Session).unwrap().is_none());
    assert!(matches!(
        paths.write_private(SecretFile::Session, b""),
        Err(ConfigError::EmptySession)
    ));
    assert!(!paths.secret_path(SecretFile::Session).exists());

    paths
        .write_private(SecretFile::Session, b"old bytes")
        .unwrap();
    paths
        .write_private(SecretFile::Session, b"new bytes")
        .unwrap();
    assert_eq!(
        paths.read_private(SecretFile::Session).unwrap(),
        Some(b"new bytes".to_vec())
    );
    assert_eq!(
        fs::metadata(paths.secret_path(SecretFile::Session))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let entries: Vec<_> = fs::read_dir(paths.data_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1, "temporary files must not remain");
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_app_directory_and_secret_target_without_writing_outside() {
    use std::os::unix::fs::symlink;

    let scratch = Scratch::new();
    let outside = scratch.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let outside_secret = outside.join("session.json");
    fs::write(&outside_secret, b"preserve").unwrap();

    let base = scratch.path().join("base");
    fs::create_dir(&base).unwrap();
    symlink(&outside, base.join("TDrive")).unwrap();
    assert!(matches!(
        ConfigDirs::from_bases(&base, &base),
        Err(ConfigError::UnsafePath)
    ));
    assert_eq!(fs::read(&outside_secret).unwrap(), b"preserve");

    let clean = ConfigDirs::from_bases(scratch.path(), scratch.path()).unwrap();
    symlink(&outside_secret, clean.secret_path(SecretFile::Session)).unwrap();
    assert!(matches!(
        clean.write_private(SecretFile::Session, b"not allowed"),
        Err(ConfigError::UnsafePath)
    ));
    assert!(matches!(
        clean.read_private(SecretFile::Session),
        Err(ConfigError::UnsafePath)
    ));
    assert_eq!(fs::read(outside_secret).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn rejects_public_secret_and_unsafe_directory() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new();
    let paths = ConfigDirs::from_bases(scratch.path(), scratch.path()).unwrap();
    let destination = paths.secret_path(SecretFile::DriveSelection);
    fs::write(&destination, b"existing public").unwrap();
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(matches!(
        paths.write_private(SecretFile::DriveSelection, b"overwrite"),
        Err(ConfigError::UnsafePermissions)
    ));
    assert_eq!(fs::read(destination).unwrap(), b"existing public");

    fs::set_permissions(paths.data_dir(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        paths.write_private(SecretFile::Session, b"secret"),
        Err(ConfigError::UnsafePermissions)
    ));
}

#[cfg(unix)]
#[test]
fn rejects_oversized_payloads_and_relative_base() {
    let scratch = Scratch::new();
    assert!(matches!(
        ConfigDirs::from_bases(Path::new("relative"), scratch.path()),
        Err(ConfigError::InvalidRoot)
    ));
    let paths = ConfigDirs::from_bases(scratch.path(), scratch.path()).unwrap();
    let bytes = vec![0xAA; 1024 * 1024 + 1];
    assert!(matches!(
        paths.write_private(SecretFile::Credentials, &bytes),
        Err(ConfigError::OversizedSecret)
    ));
    assert!(
        paths
            .read_private(SecretFile::Credentials)
            .unwrap()
            .is_none()
    );
}
