#![cfg(unix)]

use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn command_prepares_private_parts_and_receipt_without_overwriting() {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = Scratch(std::env::temp_dir().join(format!("tdrive-upload-{}-{nanos}", std::process::id())));
    DirBuilder::new().mode(0o700).create(&root.0).unwrap();
    let source = root.0.join("source");
    let output = root.0.join("parts");
    let invalid = root.0.join("invalid");
    fs::write(&source, b"abcde").unwrap();
    let run = |destination: &std::path::Path, size: &str| {
        Command::new(env!("CARGO_BIN_EXE_tdrive-inspect"))
            .arg("--prepare-upload").arg(&source).arg(destination).arg(size)
            .output().unwrap()
    };
    assert!(!run(&invalid, "0").status.success());
    assert!(!invalid.exists());
    assert!(run(&output, "2").status.success());
    assert_eq!(fs::read(output.join("0.part")).unwrap(), b"ab");
    assert_eq!(fs::read(output.join("1.part")).unwrap(), b"cd");
    assert_eq!(fs::read(output.join("2.part")).unwrap(), b"e");
    let receipt = fs::read_to_string(output.join("receipt.json")).unwrap();
    assert!(receipt.contains("36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c"));
    assert_eq!(fs::metadata(&output).unwrap().permissions().mode() & 0o077, 0);
    assert_eq!(fs::metadata(output.join("0.part")).unwrap().permissions().mode() & 0o077, 0);
    assert!(!run(&output, "3").status.success());
    assert_eq!(fs::read(output.join("0.part")).unwrap(), b"ab");
    assert_eq!(fs::read(&source).unwrap(), b"abcde");
}
