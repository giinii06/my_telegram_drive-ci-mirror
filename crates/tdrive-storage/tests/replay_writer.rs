//! Go v16 replay journaling invariants: isolated synthetic DB only.
use rusqlite::Connection;
use tdrive_storage::{
    IncomingReplayEvent, ReplayWriteError, ReplayWriteOutcome, append_replay_event,
};

const HEADER: &str = "TDX1|t=mkdir|obj=d:docs|p=|n=Docs";

fn database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE schema_version(version INTEGER PRIMARY KEY);
         INSERT INTO schema_version(version) VALUES (16);
         CREATE TABLE replay_log (
           channel_id INTEGER NOT NULL, msg_id INTEGER NOT NULL,
           op_type TEXT NOT NULL, op_payload_json TEXT NOT NULL,
           raw_header TEXT NOT NULL, first_seen_hash TEXT NOT NULL,
           actor_user_id INTEGER NOT NULL, seen_at INTEGER NOT NULL,
           PRIMARY KEY(channel_id, msg_id));
         CREATE TABLE replay_log_tamper (
           channel_id INTEGER NOT NULL, msg_id INTEGER NOT NULL,
           old_hash TEXT NOT NULL, new_hash TEXT NOT NULL,
           detected_at INTEGER NOT NULL,
           PRIMARY KEY(channel_id, msg_id));",
    )
    .unwrap();
    conn
}

fn event<'a>(header: &'a str) -> IncomingReplayEvent<'a> {
    IncomingReplayEvent {
        channel_id: 123,
        msg_id: 42,
        op_type: "mkdir",
        op_payload_json: r#"{"Type":"mkdir","Obj":"d:docs","Parent":"","Name":"Docs"}"#,
        raw_header: header,
        actor_user_id: 77,
        seen_at: 1_700_000_000,
    }
}

#[test]
fn first_seen_is_inserted_once_and_identical_redelivery_does_not_reapply() {
    let mut conn = database();
    let first = append_replay_event(&mut conn, &event(HEADER)).unwrap();
    assert_eq!(first, ReplayWriteOutcome::Inserted);
    assert_eq!(
        append_replay_event(&mut conn, &event(HEADER)).unwrap(),
        ReplayWriteOutcome::AlreadySeen
    );
    let (count, hash): (i64, String) = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM replay_log),
                    first_seen_hash FROM replay_log WHERE channel_id=123 AND msg_id=42",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(hash.len(), 64);
    assert!(hash.bytes().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn edited_telegram_header_sets_tamper_row_without_changing_canonical_op() {
    let mut conn = database();
    append_replay_event(&mut conn, &event(HEADER)).unwrap();
    let edited = "TDX1|t=mkdir|obj=d:docs|p=|n=Other";
    assert_eq!(
        append_replay_event(&mut conn, &event(edited)).unwrap(),
        ReplayWriteOutcome::TamperRecorded
    );
    let original: String = conn.query_row(
        "SELECT raw_header FROM replay_log WHERE channel_id=123 AND msg_id=42",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(original, HEADER);
    let (old_hash, new_hash, detected_at): (String, String, i64) = conn.query_row(
        "SELECT old_hash,new_hash,detected_at FROM replay_log_tamper
         WHERE channel_id=123 AND msg_id=42",
        [],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).unwrap();
    assert_ne!(old_hash, new_hash);
    assert_eq!(detected_at, 1_700_000_000);
}

#[test]
fn bad_wire_type_and_payload_never_create_replay_records() {
    let mut conn = database();
    for (header, op_type, payload) in [
        (HEADER, "rename", "{}"),
        ("TDX1|t=unknown", "mkdir", "{}"),
        (HEADER, "mkdir", "{invalid json}"),
        (HEADER, "mkdir", r#"{"Type":"rename"}"#),
        (HEADER, "mkdir", r#"{"Name":"missing operation type"}"#),
    ] {
        let mut input = event(header);
        input.op_type = op_type;
        input.op_payload_json = payload;
        assert!(append_replay_event(&mut conn, &input).is_err());
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM replay_log", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn invalid_ids_size_and_schema_reject_without_side_effect() {
    let mut conn = database();
    let mut missing_channel = event(HEADER);
    missing_channel.channel_id = 0;
    assert!(matches!(
        append_replay_event(&mut conn, &missing_channel),
        Err(ReplayWriteError::InvalidEntry)
    ));
    let mut giant = event(HEADER);
    let oversize = "x".repeat(1_048_577);
    giant.op_payload_json = &oversize;
    assert!(matches!(
        append_replay_event(&mut conn, &giant),
        Err(ReplayWriteError::InvalidEntry)
    ));
    conn.execute("UPDATE schema_version SET version=17", []).unwrap();
    assert!(matches!(
        append_replay_event(&mut conn, &event(HEADER)),
        Err(ReplayWriteError::WrongSchema)
    ));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM replay_log", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn tamper_insert_failure_rolls_back_and_original_remains_untouched() {
    let mut conn = database();
    append_replay_event(&mut conn, &event(HEADER)).unwrap();
    conn.execute_batch("DROP TABLE replay_log_tamper;").unwrap();
    let result = append_replay_event(
        &mut conn, &event("TDX1|t=mkdir|obj=d:docs|p=|n=Changed")
    );
    assert!(matches!(result, Err(ReplayWriteError::Database(_))));
    let previous: String = conn
        .query_row(
            "SELECT raw_header FROM replay_log WHERE channel_id=123 AND msg_id=42",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(previous, HEADER);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM replay_log", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
