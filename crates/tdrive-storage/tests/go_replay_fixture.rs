use std::{env, fs, path::PathBuf};
use tdrive_storage::{SnapshotError, read_replay_page_snapshot};

#[test]
fn go_replay_fixture_matches_rust_paging_hashes_and_never_modifies_db() {
    let (Some(db_path), Some(golden_path)) = (
        env::var_os("TDRIVE_GO_SQLITE_FIXTURE"),
        env::var_os("TDRIVE_GO_SQLITE_REPLAY_FIXTURE"),
    ) else {
        eprintln!("Go-generated replay oracle absent; verified in GitLab CI");
        return;
    };
    let db_path = PathBuf::from(db_path);
    let before = fs::read(&db_path).expect("Go SQLite fixture");
    let mut output = String::new();
    for (channel, after, limit) in [
        (123456_i64, 0_i64, 2_usize),
        (123456, 101, 2),
        (123456, 103, 2),
        (654321, 0, 2),
        (654321, 7, 2),
    ] {
        use std::fmt::Write as _;
        writeln!(&mut output, "CASE\t{channel}\t{after}\t{limit}").unwrap();
        for row in read_replay_page_snapshot(&db_path, channel, after, limit).unwrap() {
            writeln!(
                &mut output,
                "OP\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                row.channel_id,
                row.msg_id,
                row.op_type,
                row.op_payload_json,
                row.raw_header,
                row.first_seen_hash,
                row.actor_user_id,
                row.seen_at,
            )
            .unwrap();
        }
    }
    let oracle = fs::read_to_string(golden_path).expect("Go replay oracle");
    assert_eq!(output, oracle, "Go and Rust replay read must match exactly");
    assert_eq!(before, fs::read(&db_path).unwrap(), "Rust mutated Go snapshot");
    assert!(matches!(
        read_replay_page_snapshot(&db_path, 123456, 0, 501),
        Err(SnapshotError::InvalidReplayCursor)
    ));
    assert!(matches!(
        read_replay_page_snapshot(&db_path, 0, 0, 10),
        Err(SnapshotError::InvalidReplayCursor)
    ));
}
