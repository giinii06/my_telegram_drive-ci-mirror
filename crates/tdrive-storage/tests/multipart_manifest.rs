use tdrive_storage::{FilePart, validate_multipart_parts};

fn part(index: i64, size: i64) -> FilePart {
    FilePart { part_index: index, msg_id: index + 10, size }
}

#[test]
fn complete_ordered_stored_bytes_pass() {
    assert!(validate_multipart_parts(3, 8, &[part(0, 3), part(1, 5), part(2, 0)]).is_ok());
}

#[test]
fn gaps_truncation_and_overflow_fail_closed() {
    for (count, size, parts) in [
        (2, 8, vec![part(0, 8)]),
        (2, 8, vec![part(0, 4), part(2, 4)]),
        (2, 8, vec![part(0, 4), part(1, 3)]),
        (2, i64::MAX, vec![part(0, i64::MAX), part(1, 1)]),
        (1, 0, vec![part(0, -1)]),
        (1, -1, vec![part(0, 0)]),
    ] {
        assert!(validate_multipart_parts(count, size, &parts).is_err());
    }
}
