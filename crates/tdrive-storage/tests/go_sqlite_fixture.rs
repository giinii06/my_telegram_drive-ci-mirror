use std::{env, fs, path::PathBuf};
use tdrive_storage::{inspect_snapshot, list_folder_contents_snapshot};

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
    assert_eq!(report.count("folders"), Some(2));
    assert_eq!(report.count("files"), Some(3));
    assert_eq!(report.count("dirents"), Some(5));
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

#[test]
fn rust_listing_matches_actual_go_list_folder_contents_and_does_not_write() {
    let Some(db_path) = env::var_os("TDRIVE_GO_SQLITE_FIXTURE") else {
        eprintln!("Go fixture unavailable: test only active in CI with generated snapshot");
        return;
    };
    let Some(golden_path) = env::var_os("TDRIVE_GO_SQLITE_LISTING_FIXTURE") else {
        eprintln!("Go listing oracle unavailable: test only active in CI");
        return;
    };
    let db_path = PathBuf::from(db_path);
    let before = fs::read(&db_path).expect("read snapshot");
    let mut output = String::new();

    for (channel, parent) in [
        (123456, ""),
        (123456, "d:docs"),
        (123456, "d:archive"),
        (123456, "d:missing"),
        (654321, ""),
    ] {
        use std::fmt::Write as _;
        writeln!(&mut output, "CASE\t{channel}\t{parent}").unwrap();
        let listing = list_folder_contents_snapshot(&db_path, channel, parent)
            .expect("valid Go snapshot must allow a read-only listing");
        for folder in listing.folders {
            writeln!(
                &mut output,
                "DIR\t{}\t{}\t{}\t{}",
                folder.id, folder.name, folder.parent_id, folder.revision
            )
            .unwrap();
        }
        for file in listing.files {
            writeln!(
                &mut output,
                "FILE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                file.msg_id,
                file.content_msg_id,
                file.content_hash,
                file.revision,
                file.upload_uuid,
                file.part_count,
                file.name,
                file.size,
                file.parent_id,
                file.upload_time,
                file.uploader_id,
                file.encrypted,
                file.plaintext_size
            )
            .unwrap();
        }
    }
    let golden = fs::read_to_string(golden_path).expect("read Go-authored listing oracle");
    assert_eq!(output, golden, "Go and Rust read semantics must be identical");
    assert_eq!(fs::read(&db_path).unwrap(), before, "read-only listing must not mutate DB");
}
