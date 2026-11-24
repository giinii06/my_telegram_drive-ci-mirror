use tdrive_projection::{format_plain_multipart, parse_wire, OpType};

#[test]
fn plain_upload_headers_match_go_field_order_and_escape() {
    let result = format_plain_multipart("upload-1", "d:root", "a + b.mp4", &[2, 3], 5, 100).unwrap();
    assert_eq!(result.parts, [
        "TDX1|t=part|u=upload-1|pix=0|sz=2",
        "TDX1|t=part|u=upload-1|pix=1|sz=3",
    ]);
    assert_eq!(result.manifest, "TDX1|t=manifest|u=upload-1|p=d:root|n=a+%2B+b.mp4|pc=2|sz=5|ts=100");
    let op = parse_wire(&result.manifest).unwrap();
    assert_eq!(op.op_type, OpType::Manifest);
    assert_eq!(op.file_size, 5);
    assert_eq!(op.part_count, 2);
    assert_eq!(op.name.as_deref(), Some("a + b.mp4"));
}

#[test]
fn rejects_inconsistent_or_injected_metadata_before_send() {
    for (id, parent, name, parts, size, timestamp) in [
        ("x|t=f", "", "a", vec![1], 1, 1),
        ("x", "d:a|p=", "a", vec![1], 1, 1),
        ("x", "", "../a", vec![1], 1, 1),
        ("x", "", "a", vec![0], 0, 1),
        ("x", "", "a", vec![2, 3], 4, 1),
        ("x", "", "a", vec![i64::MAX, 1], i64::MAX, 1),
        ("x", "", "a", vec![1], 1, -1),
    ] {
        assert!(format_plain_multipart(id, parent, name, &parts, size, timestamp).is_err());
    }
}
