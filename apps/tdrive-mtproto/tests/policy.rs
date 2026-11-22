#![cfg(unix)]
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_mtproto::policy::{
    CliAction, PolicyError, parse_action, validate_api_credentials, validate_download_path,
    validate_session_path, validate_source,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct PrivateFixture(PathBuf);
impl PrivateFixture {
    fn new() -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tdrive-mtproto-policy-{}-{nanos}-{}",
            std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn path(&self, file: &str) -> PathBuf { self.0.join(file) }
}
impl Drop for PrivateFixture {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}
fn args(values: &[&str]) -> Vec<String> { values.iter().map(|s| (*s).into()).collect() }

#[test]
fn login_and_read_commands_are_user_account_only_and_bounded() {
    assert_eq!(parse_action(&args(&["login"])).unwrap(), CliAction::Login);
    assert_eq!(parse_action(&args(&["status"])).unwrap(), CliAction::Status);
    assert_eq!(parse_action(&args(&["dialogs"])).unwrap(), CliAction::Dialogs);
    assert_eq!(parse_action(&args(&["inbox-stats"])).unwrap(), CliAction::InboxStats);
    assert!(matches!(parse_action(&args(&["ingest", "@owner", "100"])),
        Ok(CliAction::Ingest { limit: 100, .. })));
    assert!(matches!(
        parse_action(&args(&["history", "@owner", "100"])),
        Ok(CliAction::History { limit: 100, .. })
    ));
    for values in [
        vec!["history", "@owner", "0"],
        vec!["ingest", "@owner", "0"],
        vec!["ingest", "@owner", "101"],
        vec!["ingest", "@owner"],
        vec!["history", "@owner", "101"],
        vec!["history", "a/b", "10"],
        vec!["login", "phone-number-on-argv"],
        vec!["upload", "owner"],
        vec!["download", "@owner", "0", "/tmp/file"],
    ] {
        assert_eq!(parse_action(&args(&values)), Err(PolicyError::InvalidArguments));
    }
}

#[test]
fn explicit_remote_upload_or_download_requires_absolute_files() {
    assert!(matches!(
        parse_action(&args(&["upload", "@owner", "/tmp/test.bin"])),
        Ok(CliAction::Upload { .. })
    ));
    assert!(matches!(
        parse_action(&args(&["download", "@owner", "42", "/tmp/test.bin"])),
        Ok(CliAction::Download { message_id: 42, .. })
    ));
    for values in [
        vec!["upload", "@owner", "relative/file.bin"],
        vec!["download", "@owner", "42", "relative/out.bin"],
        vec!["upload", "@owner", "/tmp/../etc/passwd"],
    ] {
        assert_eq!(parse_action(&args(&values)), Err(PolicyError::InvalidArguments));
    }
}

#[test]
fn invalid_api_credentials_and_untrusted_session_dirs_are_rejected() {
    assert_eq!(
        validate_api_credentials("14948", "0123456789abcdef0123456789abcdef"),
        Ok(14948)
    );
    for (id, secret) in [
        ("0", "0123456789abcdef0123456789abcdef"),
        ("bad", "0123456789abcdef0123456789abcdef"),
        ("42", "test"),
        ("42", "0123456789abcdef0123456789abcdef\n"),
    ] {
        assert_eq!(
            validate_api_credentials(id, secret), Err(PolicyError::InvalidCredentials)
        );
    }
    let fixture = PrivateFixture::new();
    assert!(validate_session_path(&fixture.path("session.sqlite")).is_ok());
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        validate_session_path(&fixture.path("session.sqlite")),
        Err(PolicyError::UnsafePath)
    );
}

#[test]
fn session_and_file_symlinks_are_rejected_without_telegrams() {
    let fixture = PrivateFixture::new();
    let file = fixture.path("real.bin");
    let mut writer = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&file).unwrap();
    writer.write_all(b"test").unwrap();
    assert!(validate_source(&file).is_ok());
    let link = fixture.path("linked.bin");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    assert_eq!(validate_source(&link), Err(PolicyError::UnsafePath));
    let target = fixture.path("output.bin");
    assert!(validate_download_path(&target).is_ok());
    fs::write(&target, b"already exists").unwrap();
    assert_eq!(validate_download_path(&target), Err(PolicyError::UnsafePath));
    let session_link = fixture.path("session.sqlite");
    std::os::unix::fs::symlink(&file, &session_link).unwrap();
    assert_eq!(validate_session_path(&session_link), Err(PolicyError::UnsafePath));
}

#[test]
fn download_rejects_symlink_directory_and_missing_parent() {
    let fixture = PrivateFixture::new();
    let real_dir = fixture.path("destination");
    fs::create_dir(&real_dir).unwrap();
    let link_dir = fixture.path("linked-directory");
    std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();
    assert_eq!(
        validate_download_path(&link_dir.join("file.bin")),
        Err(PolicyError::UnsafePath),
    );
    assert_eq!(
        validate_download_path(&fixture.path("missing-parent").join("file.bin")),
        Err(PolicyError::UnsafePath),
    );
}

