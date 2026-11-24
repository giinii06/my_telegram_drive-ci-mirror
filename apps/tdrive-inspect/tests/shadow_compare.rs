use std::{env, fs, process::Command};

#[test]
fn missing_or_invalid_snapshot_fails_without_creating_files() {
    let dir = env::temp_dir().join(format!("tdrive-shadow-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let missing = dir.join("missing.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg("--compare-snapshots").arg(&missing).arg(&missing).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("snapshot comparison failed"));
    assert!(!missing.exists());
    fs::remove_dir(&dir).unwrap();
}

#[test]
fn identical_go_fixture_has_equal_core_tables_when_available() {
    let Some(path) = env::var_os("TDRIVE_GO_SQLITE_FIXTURE") else { return };
    let before = fs::read(&path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg("--compare-snapshots").arg(&path).arg(&path).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("core tables match"));
    assert_eq!(fs::read(&path).unwrap(), before);
}
