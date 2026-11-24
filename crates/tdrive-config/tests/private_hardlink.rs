#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_config::{ConfigDirs, ConfigError, SecretFile};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn multiply_linked_session_is_not_read_or_replaced() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Scratch(std::env::temp_dir().join(format!(
        "tdrive-hardlink-{}-{now}",
        std::process::id()
    )));
    fs::create_dir(&root.0).unwrap();
    let dirs = ConfigDirs::from_bases(&root.0, &root.0).unwrap();
    let outside = root.0.join("other-session");
    fs::write(&outside, b"original secret").unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
    fs::hard_link(&outside, dirs.secret_path(SecretFile::Session)).unwrap();

    assert!(matches!(
        dirs.read_private(SecretFile::Session),
        Err(ConfigError::UnsafePath)
    ));
    assert!(matches!(
        dirs.write_private(SecretFile::Session, b"replacement"),
        Err(ConfigError::UnsafePath)
    ));
    assert_eq!(fs::read(&outside).unwrap(), b"original secret");
    assert_eq!(
        fs::read(dirs.secret_path(SecretFile::Session)).unwrap(),
        b"original secret"
    );
}
