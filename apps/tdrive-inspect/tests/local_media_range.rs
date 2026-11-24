use std::{fs, path::PathBuf, process::Command, time::{SystemTime, UNIX_EPOCH}};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

#[test]
fn media_range_stdout_contains_only_requested_bytes() {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = Scratch(std::env::temp_dir().join(format!("tdrive-range-{}-{nanos}", std::process::id())));
    fs::create_dir(&root.0).unwrap();
    let source = root.0.join("media");
    fs::write(&source, b"0123456789").unwrap();
    let run = |range: &str| Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
        .arg("--read-media-range").arg(&source).arg(range).output().unwrap();
    for (range, expected) in [
        ("", b"0123456789".as_slice()),
        ("bytes=2-4", b"234".as_slice()),
        ("bytes=-2", b"89".as_slice()),
        ("bytes=8-", b"89".as_slice()),
    ] {
        let result = run(range);
        assert!(result.status.success(), "{range}");
        assert_eq!(result.stdout, expected);
    }
    for range in ["bytes=10-", "bytes=0-1,2-3", "nonsense"] {
        let result = run(range);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
    assert_eq!(fs::read(&source).unwrap(), b"0123456789");
}
