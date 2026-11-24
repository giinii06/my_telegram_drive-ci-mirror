use rusqlite::Connection;
use tdrive_storage::{ReplayWriteOutcome, project_plain_folder};

fn db() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(
        "CREATE TABLE schema_version(version INTEGER PRIMARY KEY);
         INSERT INTO schema_version VALUES(16);
         CREATE TABLE replay_log(channel_id INTEGER,msg_id INTEGER,op_type TEXT,
           op_payload_json TEXT,raw_header TEXT,first_seen_hash TEXT,
           actor_user_id INTEGER,seen_at INTEGER,PRIMARY KEY(channel_id,msg_id));
         CREATE TABLE replay_log_tamper(channel_id INTEGER,msg_id INTEGER,
           old_hash TEXT,new_hash TEXT,detected_at INTEGER,PRIMARY KEY(channel_id,msg_id));
         CREATE TABLE folders(channel_id INTEGER,id TEXT,name TEXT,parent_id TEXT,
           tombstoned INTEGER DEFAULT 0,revision INTEGER DEFAULT 1,PRIMARY KEY(channel_id,id));
         CREATE TABLE dirents(channel_id INTEGER,object_id TEXT,object_kind TEXT,parent_id TEXT,
           display_name TEXT,name_key TEXT,revision INTEGER,tombstoned INTEGER,
           PRIMARY KEY(channel_id,object_id));
         CREATE UNIQUE INDEX live_names ON dirents(channel_id,parent_id,name_key) WHERE tombstoned=0;",
    ).unwrap();
    c
}

#[test]
fn mkdir_is_atomic_idempotent_and_channel_scoped() {
    let mut c = db();
    let header = "TDX1|t=mkdir|obj=d:one|p=|n=Docs";
    assert_eq!(project_plain_folder(&mut c, 1, 10, header, 7, 100).unwrap(), ReplayWriteOutcome::Inserted);
    assert_eq!(project_plain_folder(&mut c, 1, 10, header, 7, 100).unwrap(), ReplayWriteOutcome::AlreadySeen);
    project_plain_folder(&mut c, 2, 10, header, 7, 100).unwrap();
    let row: (String, String, i64) = c.query_row(
        "SELECT display_name,name_key,revision FROM dirents WHERE channel_id=1 AND object_id='d:one'",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(row, ("Docs".into(), "docs".into(), 1));
    project_plain_folder(&mut c, 1, 11, "TDX1|t=mkdir|obj=d:one|p=|n=Other", 7, 100).unwrap();
    let name: String = c.query_row("SELECT name FROM folders WHERE channel_id=1 AND id='d:one'", [], |r| r.get(0)).unwrap();
    assert_eq!(name, "Docs");
}

#[test]
fn collisions_unsupported_names_and_db_failure_leave_no_journal_row() {
    let mut c = db();
    project_plain_folder(&mut c, 1, 10, "TDX1|t=mkdir|obj=d:one|p=|n=Docs", 7, 100).unwrap();
    for header in [
        "TDX1|t=mkdir|obj=d:two|p=|n=docs",
        "TDX1|t=mkdir|obj=d:two|p=|n=CON.txt",
        "TDX1|t=mkdir|obj=d:two|p=|n=%ED%95%9C",
    ] {
        assert!(project_plain_folder(&mut c, 1, 11, header, 7, 100).is_err());
    }
    c.execute_batch("CREATE TRIGGER fail_dirent BEFORE INSERT ON dirents BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    assert!(project_plain_folder(&mut c, 1, 11, "TDX1|t=mkdir|obj=d:two|p=|n=Other", 7, 100).is_err());
    for table in ["folders","dirents","replay_log"] {
        let count: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }
}
