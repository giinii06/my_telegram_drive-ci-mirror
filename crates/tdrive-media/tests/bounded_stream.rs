use std::io::{Cursor, ErrorKind};
use tdrive_media::copy_range;

#[test]
fn sends_only_requested_bytes() {
    let mut source = Cursor::new(b"0123456789".to_vec());
    let mut body = Vec::new();
    let result = copy_range(&mut source, &mut body, "bytes=2-4", 10).unwrap();
    assert_eq!(result.response_code(), 206);
    assert_eq!(body, b"234");
}

#[test]
fn invalid_range_does_not_read_or_write() {
    let mut source = Cursor::new(b"abc".to_vec());
    let mut body = Vec::new();
    let result = copy_range(&mut source, &mut body, "bytes=5-", 3).unwrap();
    assert_eq!(result.response_code(), 416);
    assert_eq!(source.position(), 0);
    assert!(body.is_empty());
}

#[test]
fn truncated_and_negative_sources_fail() {
    let mut source = Cursor::new(b"abc".to_vec());
    let mut body = Vec::new();
    assert_eq!(copy_range(&mut source, &mut body, "", 4).unwrap_err().kind(), ErrorKind::UnexpectedEof);
    assert_eq!(copy_range(&mut source, &mut body, "", -1).unwrap_err().kind(), ErrorKind::InvalidInput);
}
