use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, process::Command};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("tdrive-shadow-{}-{now}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_or_invalid_snapshot_fails_without_creating_files() {
    let dir = Scratch::new();
    let missing = dir.0.join("missing.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg("--compare-snapshots")
        .arg(&missing)
        .arg(&missing)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("snapshot comparison failed"));
    assert!(!missing.exists());
}

#[test]
fn go_fixture_comparison_detects_changed_content_when_available() {
    let Some(path) = env::var_os("TDRIVE_GO_SQLITE_FIXTURE") else {
        return;
    };
    let before = fs::read(&path).unwrap();
    let dir = Scratch::new();
    let copy = dir.0.join("copy.sqlite");
    fs::copy(&path, &copy).unwrap();
    let compare = || {
        Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
            .arg("--compare-snapshots")
            .arg(&path)
            .arg(&copy)
            .output()
            .unwrap()
    };
    let output = compare();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("core tables match"));

    let conn = rusqlite::Connection::open(&copy).unwrap();
    assert!(conn.execute("UPDATE channels SET title=title || '-shadow-test'", []).unwrap() > 0);
    drop(conn);
    let changed = fs::read(&copy).unwrap();
    let output = compare();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("core table differs: channels"));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&copy).unwrap(), changed);
}
