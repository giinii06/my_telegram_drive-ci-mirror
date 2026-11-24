#![cfg(unix)]
use std::io::{Cursor, ErrorKind};
use tdrive_mtproto::multipart_download::{copy_exact_part, validate_manifest};
use tdrive_mtproto::policy::{CliAction, parse_action};
use tdrive_storage::FilePart;

fn part(index: i64, message: i64, size: i64) -> FilePart {
    FilePart {
        part_index: index,
        msg_id: message,
        size,
    }
}

#[test]
fn multipart_command_requires_explicit_channel_and_digest() {
    let hash = "a".repeat(64);
    let args = |channel: &str, message: &str, size: &str| {
        ["download-multipart-verified", "@owner", "/tmp/snapshot.sqlite",
         channel, message, "/tmp/output", size, &hash]
            .iter().map(|s| s.to_string()).collect::<Vec<_>>()
    };
    assert!(matches!(parse_action(&args("42", "10", "5")),
        Ok(CliAction::DownloadMultipartVerified { channel_id: 42, file_message_id: 10, .. })));
    for (channel, message, size) in [("0", "10", "5"), ("42", "+10", "5"), ("42", "10", "-5")] {
        assert!(parse_action(&args(channel, message, size)).is_err());
    }
}

#[test]
fn manifest_rejects_unrepresentable_telegram_message_ids_and_wrong_lengths() {
    assert!(validate_manifest(&[part(0, 10, 2), part(1, 11, 3)], 5).is_ok());
    assert!(validate_manifest(&[part(0, i64::from(i32::MAX) + 1, 5)], 5).is_err());
    assert!(validate_manifest(&[part(0, 10, 4)], 5).is_err());
    assert!(validate_manifest(&[part(1, 10, 5)], 5).is_err());
    assert!(validate_manifest(&[], 0).is_err());
}

#[test]
fn part_copy_never_accepts_short_or_oversized_content() {
    let mut staged = Vec::new();
    copy_exact_part(&mut Cursor::new(b"abc"), &mut staged, 3).unwrap();
    assert_eq!(staged, b"abc");
    assert_eq!(copy_exact_part(&mut Cursor::new(b"a"), &mut Vec::new(), 2).unwrap_err().kind(), ErrorKind::UnexpectedEof);
    assert_eq!(copy_exact_part(&mut Cursor::new(b"abc"), &mut Vec::new(), 2).unwrap_err().kind(), ErrorKind::InvalidData);
}
