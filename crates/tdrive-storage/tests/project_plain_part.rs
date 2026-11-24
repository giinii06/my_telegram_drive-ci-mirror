use rusqlite::Connection;
use tdrive_storage::{ReplayWriteError, ReplayWriteOutcome, project_plain_part};

fn db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE schema_version(version INTEGER PRIMARY KEY);
         INSERT INTO schema_version VALUES(16);
         CREATE TABLE replay_log(channel_id INTEGER,msg_id INTEGER,op_type TEXT,
           op_payload_json TEXT,raw_header TEXT,first_seen_hash TEXT,
           actor_user_id INTEGER,seen_at INTEGER,PRIMARY KEY(channel_id,msg_id));
         CREATE TABLE replay_log_tamper(channel_id INTEGER,msg_id INTEGER,
           old_hash TEXT,new_hash TEXT,detected_at INTEGER,PRIMARY KEY(channel_id,msg_id));
         CREATE TABLE file_parts(channel_id INTEGER,upload_uuid TEXT,part_index INTEGER,
           msg_id INTEGER,size INTEGER,PRIMARY KEY(channel_id,upload_uuid,part_index));",
    ).unwrap();
    conn
}

const PART: &str = "TDX1|t=part|u=upload-1|pix=0|sz=42";

#[test]
fn first_part_wins_and_redelivery_does_not_change_it() {
    let mut conn = db();
    assert_eq!(project_plain_part(&mut conn, 1, 10, PART, 7, 100).unwrap(), ReplayWriteOutcome::Inserted);
    assert_eq!(project_plain_part(&mut conn, 1, 10, PART, 7, 100).unwrap(), ReplayWriteOutcome::AlreadySeen);
    assert_eq!(project_plain_part(&mut conn, 1, 11, "TDX1|t=part|u=upload-1|pix=0|sz=99", 7, 101).unwrap(), ReplayWriteOutcome::Inserted);
    assert_eq!(project_plain_part(&mut conn, 1, 10, "TDX1|t=part|u=upload-1|pix=0|sz=99", 7, 102).unwrap(), ReplayWriteOutcome::TamperRecorded);
    let row: (i64, i64) = conn.query_row("SELECT msg_id,size FROM file_parts", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(row, (10, 42));
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM replay_log", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn projection_failure_rolls_back_journal_and_out_of_order_is_rejected() {
    let mut conn = db();
    conn.execute_batch("CREATE TRIGGER fail_part BEFORE INSERT ON file_parts BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(project_plain_part(&mut conn, 1, 10, PART, 7, 100).is_err());
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM replay_log", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
    conn.execute_batch("DROP TRIGGER fail_part;").unwrap();
    project_plain_part(&mut conn, 1, 10, PART, 7, 100).unwrap();
    assert!(matches!(project_plain_part(&mut conn, 1, 9, PART, 7, 100), Err(ReplayWriteError::OutOfOrder)));
    assert!(project_plain_part(&mut conn, 1, 12, "TDX1|t=mkdir|obj=d:x|p=|n=x", 7, 100).is_err());
    conn.execute("DELETE FROM file_parts", []).unwrap();
    assert!(matches!(project_plain_part(&mut conn, 1, 10, PART, 7, 100), Err(ReplayWriteError::UnappliedReplay)));
}

#[test]
fn future_or_inconsistent_existing_part_cannot_commit_new_replay_row() {
    for (id, size) in [(20, 42), (10, 99), (5, -1)] {
        let mut conn = db();
        conn.execute("INSERT INTO file_parts VALUES(1,'upload-1',0,?1,?2)", [id, size]).unwrap();
        assert!(matches!(
            project_plain_part(&mut conn, 1, 10, PART, 7, 100),
            Err(ReplayWriteError::UnappliedReplay)
        ));
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM replay_log", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
        let row: (i64, i64) = conn.query_row("SELECT msg_id,size FROM file_parts", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(row, (id,size));
    }
    let mut conn = db();
    conn.execute("INSERT INTO file_parts VALUES(1,'upload-1',0,5,41)", []).unwrap();
    assert_eq!(project_plain_part(&mut conn, 1, 10, PART, 7, 100).unwrap(), ReplayWriteOutcome::Inserted);
    assert_eq!(project_plain_part(&mut conn, 1, 10, PART, 7, 100).unwrap(), ReplayWriteOutcome::AlreadySeen);
}
