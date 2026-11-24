use rusqlite::Connection;
use tdrive_storage::{project_plain_folder, ReplayWriteOutcome};

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
         CREATE TABLE folders(channel_id INTEGER,id TEXT,name TEXT,parent_id TEXT,
           revision INTEGER DEFAULT 1,tombstoned INTEGER DEFAULT 0,PRIMARY KEY(channel_id,id));
         CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,object_kind TEXT,parent_id TEXT,
           display_name TEXT,name_key TEXT,revision INTEGER,tombstoned INTEGER,
           PRIMARY KEY(channel_id,object_id));
         CREATE UNIQUE INDEX idx_dirents_live_sibling_name ON dirents(channel_id,parent_id,name_key)
           WHERE tombstoned=0;",
    ).unwrap();
    conn
}

#[test]
fn mkdir_and_rmdir_are_atomic_first_write_wins_and_redeliverable() {
    let mut conn = db();
    let mkdir = "TDX1|t=mkdir|obj=d:x|p=|n=Docs";
    let rmdir = "TDX1|t=rmdir|obj=d:x";
    assert_eq!(project_plain_folder(&mut conn, 1, 10, mkdir, 7, 100).unwrap(), ReplayWriteOutcome::Inserted);
    assert_eq!(project_plain_folder(&mut conn, 1, 10, mkdir, 7, 100).unwrap(), ReplayWriteOutcome::AlreadySeen);
    project_plain_folder(&mut conn, 1, 11, "TDX1|t=mkdir|obj=d:x|p=|n=Other", 7, 101).unwrap();
    let row: (String, String, i64, i64) = conn.query_row("SELECT display_name,name_key,revision,tombstoned FROM dirents", [],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
    assert_eq!(row, ("Docs".into(),"docs".into(),1,0));
    project_plain_folder(&mut conn, 1, 12, rmdir, 7, 102).unwrap();
    project_plain_folder(&mut conn, 1, 13, rmdir, 7, 103).unwrap();
    assert_eq!(project_plain_folder(&mut conn, 1, 12, rmdir, 7, 104).unwrap(), ReplayWriteOutcome::AlreadySeen);
    let row: (i64,i64,i64,i64) = conn.query_row("SELECT f.revision,f.tombstoned,d.revision,d.tombstoned FROM folders f JOIN dirents d ON f.channel_id=d.channel_id AND f.id=d.object_id", [],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
    assert_eq!(row, (2,1,2,1));
}

#[test]
fn conflict_and_inconsistent_dirent_roll_back_journal() {
    let mut conn = db();
    project_plain_folder(&mut conn, 1, 10, "TDX1|t=mkdir|obj=d:x|p=|n=Docs", 7, 100).unwrap();
    for wire in ["TDX1|t=mkdir|obj=d:y|p=|n=docs", "TDX1|t=mkdir|obj=d:y|p=|n=CON.txt"] {
        assert!(project_plain_folder(&mut conn, 1, 11, wire, 7, 101).is_err());
    }
    conn.execute("UPDATE dirents SET revision=2", []).unwrap();
    assert!(project_plain_folder(&mut conn, 1, 12, "TDX1|t=rmdir|obj=d:x", 7, 102).is_err());
    assert_eq!(conn.query_row("SELECT count(*) FROM replay_log", [], |r| r.get::<_,i64>(0)).unwrap(), 1);
    assert_eq!(conn.query_row("SELECT tombstoned FROM folders", [], |r| r.get::<_,i64>(0)).unwrap(), 0);
}
