use tdrive_projection::{WireError, format_simple_wire, parse_wire};

#[test]
fn multipart_metadata_survives_parsing() {
    let part = parse_wire("TDX1|t=part|u=upload-42|pix=3|sz=4096").unwrap();
    assert_eq!(part.upload_uuid.as_deref(), Some("upload-42"));
    assert_eq!(part.part_index, 3);
    assert_eq!(part.file_size, 4096);
    let manifest = parse_wire("TDX1|t=manifest|u=upload-42|p=|n=movie|pc=4|sz=16384|enc=1|psz=16000").unwrap();
    assert_eq!(manifest.upload_uuid.as_deref(), Some("upload-42"));
    assert_eq!(manifest.part_count, 4);
    assert_eq!(manifest.plaintext_size, 16000);
    assert_eq!(manifest.encryption_version, 1);
}

#[test]
fn replacement_and_relocation_keep_cas_and_retention_values() {
    let replacement = parse_wire("TDX1|t=freplace|v=1|oid=replace|obj=f:10|rev=7|u=upload-42|pc=4|hash=abc%2Bdef|retain=999|sz=16384").unwrap();
    assert_eq!(replacement.protocol_version, 1);
    assert_eq!(replacement.expected_revision, 7);
    assert_eq!(replacement.retained_until, 999);
    assert_eq!(replacement.content_hash.as_deref(), Some("abc+def"));
    assert_eq!(replacement.upload_uuid.as_deref(), Some("upload-42"));

    let moved = parse_wire("TDX1|t=relocate|v=1|oid=move|obj=f:10|p=d:folder|n=name|rev=7|ow=1|dst=f:11|drev=8|del=100|purge=200").unwrap();
    assert_eq!(moved.expected_revision, 7);
    assert!(moved.overwrite);
    assert_eq!(moved.destination_object.as_deref(), Some("f:11"));
    assert_eq!(moved.expected_destination_revision, 8);
    assert_eq!(moved.deleted_at, 100);
    assert_eq!(moved.purge_after, 200);
}

#[test]
fn trash_restore_and_sealed_name_fields_are_not_discarded() {
    let trashed = parse_wire("TDX1|t=trash|v=1|oid=trash|obj=f:10|rev=9|del=100|purge=200").unwrap();
    assert_eq!((trashed.expected_revision, trashed.deleted_at, trashed.purge_after), (9, 100, 200));
    let restored = parse_wire("TDX1|t=untrash|v=1|oid=restore|obj=f:10|p=|n=_|nenc=v1:AA|kid=2|enc=1|ev=3|rev=10").unwrap();
    assert_eq!(restored.expected_revision, 10);
    assert_eq!(restored.name_key_version, 2);
    assert_eq!(restored.encryption_version, 3);
    let hard_delete = parse_wire("TDX1|t=harddel|v=1|oid=delete|obj=f:10|rev=11").unwrap();
    assert_eq!(hard_delete.expected_revision, 11);
}

#[test]
fn simple_formatter_never_silently_discards_encryption_metadata() {
    let operation = parse_wire("TDX1|t=f|p=|n=a|psz=4|ev=2").unwrap();
    assert_eq!(format_simple_wire(&operation), Err(WireError::UnsupportedFormat));
}
