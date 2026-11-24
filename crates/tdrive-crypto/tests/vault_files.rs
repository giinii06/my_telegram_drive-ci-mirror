#![cfg(unix)]
use std::fs::{self, DirBuilder};
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_crypto::{VaultSession, encode_key_check, wrap_master_key};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn authenticated_session_encrypts_and_decrypts_without_exporting_key() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Scratch(std::env::temp_dir().join(format!(
        "tdrive-vault-file-{}-{now}",
        std::process::id()
    )));
    DirBuilder::new().mode(0o700).create(&root.0).unwrap();
    let plain = root.0.join("plain");
    let sealed = root.0.join("sealed");
    let output = root.0.join("output");
    fs::write(&plain, b"vault content").unwrap();
    let mut session = VaultSession::new();
    assert!(session.encrypt_file(&plain, &sealed).is_err());
    assert!(!sealed.exists());
    let master = [7; 32];
    let kek = [8; 32];
    session
        .unlock(
            &wrap_master_key(&master, &kek).unwrap(),
            &encode_key_check(&master).unwrap(),
            &kek,
        )
        .unwrap();
    session.encrypt_file(&plain, &sealed).unwrap();
    assert!(session.encrypt_file(&plain, &sealed).is_err());
    session.decrypt_file(&sealed, &output).unwrap();
    assert_eq!(fs::read(&output).unwrap(), b"vault content");
    fs::remove_file(&output).unwrap();
    let mut corrupt = fs::read(&sealed).unwrap();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&sealed, corrupt).unwrap();
    assert!(session.decrypt_file(&sealed, &output).is_err());
    assert!(!output.exists());
    session.lock();
    assert!(session.decrypt_file(&sealed, &output).is_err());
    assert!(!output.exists());
}
