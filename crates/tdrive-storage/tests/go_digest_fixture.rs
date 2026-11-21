use std::{env, fs, path::PathBuf};
use tdrive_storage::snapshot_table_digests;

#[test]
fn go_v16_all_core_table_rows_match_rust_sha256_digests() {
    let (Some(db_path), Some(oracle_path)) = (
        env::var_os("TDRIVE_GO_SQLITE_FIXTURE"),
        env::var_os("TDRIVE_GO_SQLITE_DIGEST_FIXTURE"),
    ) else {
        eprintln!("Go fixture and Go digest oracle generated in GitLab CI only");
        return;
    };
    let db_path = PathBuf::from(db_path);
    let before = fs::read(&db_path).expect("read detached Go database");
    let mut result = String::new();
    for item in snapshot_table_digests(&db_path).unwrap() {
        use std::fmt::Write as _;
        writeln!(
            &mut result,
            "{}\t{}\t{}",
            item.table, item.row_count, item.sha256
        )
        .unwrap();
    }
    assert_eq!(
        result,
        fs::read_to_string(oracle_path).expect("Go generated digest oracle")
    );
    assert_eq!(
        before,
        fs::read(&db_path).unwrap(),
        "Rust must not write snapshot"
    );
}
