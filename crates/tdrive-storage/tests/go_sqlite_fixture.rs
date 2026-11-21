use std::{env, fs, path::PathBuf};
use tdrive_storage::inspect_snapshot;

#[test]
fn go_v16_fixture_is_inspected_without_changing_database_bytes() {
    let Some(path) = env::var_os("TDRIVE_GO_SQLITE_FIXTURE") else {
        eprintln!("set TDRIVE_GO_SQLITE_FIXTURE to execute the Go-generated artifact test");
        return;
    };
    let path = PathBuf::from(path);
    let before = fs::read(&path).expect("read fixture before Rust inspection");
    let report = inspect_snapshot(&path).expect("Rust must parse Go v16 fixture");
    assert_eq!(report.schema_version, 16);
    assert_eq!(report.count("channels"), Some(1));
    assert_eq!(report.count("folders"), Some(1));
    assert_eq!(report.count("files"), Some(1));
    assert_eq!(report.count("dirents"), Some(2));
    assert_eq!(report.count("file_revisions"), Some(1));
    let after = fs::read(&path).expect("read fixture after Rust inspection");
    assert_eq!(before, after, "inspector must not change SQLite DB bytes");
}

#[test]
fn missing_snapshot_is_not_created() {
    let path = std::env::temp_dir().join(format!(
        "tdrive-should-not-create-{}-{}.sqlite",
        std::process::id(),
        "phase04"
    ));
    if path.exists() {
        // Never remove a file whose ownership we cannot establish.
        return;
    }
    assert!(inspect_snapshot(&path).is_err());
    assert!(!path.exists());
}
