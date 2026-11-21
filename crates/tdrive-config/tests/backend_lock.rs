#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicU64, Ordering},
};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use tdrive_config::{BackendLock, ConfigDirs, ConfigError};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("tdrive-lock-test-{}-{now}-{n}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn config(&self) -> ConfigDirs {
        ConfigDirs::from_bases(&self.0, &self.0).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rust_lock_is_go_compatible_json_and_exclusive() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new();
    let config = scratch.config();
    let lock = BackendLock::acquire(&config, "daemon").unwrap();
    let info = lock.info();
    assert_eq!(info.role, "daemon");
    assert_eq!(info.pid, std::process::id());
    let bytes = fs::read_to_string(lock.path()).unwrap();
    let want = format!(
        "{{\"role\":\"daemon\",\"pid\":{},\"started_at\":{}}}\n",
        info.pid, info.started_at
    );
    assert_eq!(bytes, want);
    assert_eq!(
        fs::metadata(lock.path()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(matches!(
        BackendLock::acquire(&config, "gui"),
        Err(ConfigError::LockHeld)
    ));
    lock.release().unwrap();
    let next = BackendLock::acquire(&config, "gui").unwrap();
    assert_eq!(next.info().role, "gui");
}

#[test]
fn existing_go_lock_is_never_removed_or_overwritten() {
    let scratch = Scratch::new();
    let config = scratch.config();
    let path = config.data_dir().join("backend.lock");
    let existing = b"{\"role\":\"daemon\",\"pid\":1,\"started_at\":7}\n";
    fs::write(&path, existing).unwrap();
    assert!(matches!(
        BackendLock::acquire(&config, "gui"),
        Err(ConfigError::LockHeld)
    ));
    assert_eq!(fs::read(path).unwrap(), existing);
}

#[test]
fn rejects_unsafe_role_and_lock_symlink() {
    use std::os::unix::fs::symlink;

    let scratch = Scratch::new();
    let config = scratch.config();
    assert!(matches!(
        BackendLock::acquire(&config, "a\\\"},\\\"pid\\\":0"),
        Err(ConfigError::InvalidLockRole)
    ));
    let outside = scratch.0.join("outside");
    fs::write(&outside, b"do not touch").unwrap();
    symlink(&outside, config.data_dir().join("backend.lock")).unwrap();
    assert!(matches!(
        BackendLock::acquire(&config, "daemon"),
        Err(ConfigError::LockHeld)
    ));
    assert_eq!(fs::read(outside).unwrap(), b"do not touch");
}

#[test]
fn competing_rust_process_roles_only_one_wins() {
    let scratch = Scratch::new();
    let config = Arc::new(scratch.config());
    let start = Arc::new(Barrier::new(12));
    let done = Arc::new(Barrier::new(12));
    let mut threads = Vec::new();
    for _ in 0..12 {
        let dirs = Arc::clone(&config);
        let start = Arc::clone(&start);
        let done = Arc::clone(&done);
        threads.push(thread::spawn(move || {
            start.wait();
            let result = BackendLock::acquire(&dirs, "daemon");
            let success = result.is_ok();
            done.wait();
            success
        }));
    }
    let wins = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(wins, 1);
    assert!(!config.data_dir().join("backend.lock").exists());
}
