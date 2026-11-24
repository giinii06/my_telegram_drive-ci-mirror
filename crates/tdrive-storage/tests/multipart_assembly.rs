use sha2::{Digest, Sha256};
use std::io::{self, Cursor};
use tdrive_storage::{FilePart, assemble_multipart};

fn parts() -> [FilePart; 2] {
    [
        FilePart { part_index: 0, msg_id: 10, size: 2 },
        FilePart { part_index: 1, msg_id: 11, size: 3 },
    ]
}

#[test]
fn ordered_parts_are_assembled_and_digest_checked() {
    let mut staged = Vec::new();
    let hash: [u8; 32] = Sha256::digest(b"abcde").into();
    let mut requested = Vec::new();
    let bytes = assemble_multipart(2, 5, &parts(), hash, &mut staged, |part| {
        requested.push(part.msg_id);
        Ok(Cursor::new(if part.msg_id == 10 { b"ab".to_vec() } else { b"cde".to_vec() }))
    }).unwrap();
    assert_eq!(requested, [10, 11]);
    assert_eq!(bytes, 5);
    assert_eq!(staged, b"abcde");
}

#[test]
fn invalid_manifest_does_not_open_or_write_any_part() {
    let mut staged = Vec::new();
    let mut invalid = parts();
    invalid[1].part_index = 2;
    let result = assemble_multipart(2, 5, &invalid, [0; 32], &mut staged,
        |_| -> io::Result<Cursor<Vec<u8>>> { panic!("invalid manifest opened") });
    assert!(result.is_err());
    assert!(staged.is_empty());
}

#[test]
fn short_extra_wrong_hash_and_source_errors_never_return_success() {
    let hash: [u8; 32] = Sha256::digest(b"abcde").into();
    for first in [b"a".as_slice(), b"abc".as_slice(), b"xx".as_slice()] {
        let mut staged = Vec::new();
        assert!(assemble_multipart(2, 5, &parts(), hash, &mut staged, |part| {
            Ok(Cursor::new(if part.msg_id == 10 { first.to_vec() } else { b"cde".to_vec() }))
        }).is_err());
    }
    let mut staged = Vec::new();
    assert!(assemble_multipart(2, 5, &parts(), hash, &mut staged,
        |_| -> io::Result<Cursor<Vec<u8>>> { Err(io::Error::other("synthetic remote failure")) }).is_err());
}
