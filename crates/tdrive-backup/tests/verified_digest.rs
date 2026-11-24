use tdrive_backup_core::{Candidate, GuardError, Original, RemoteFile, UploadStatus};

fn original() -> Original {
    Original { size: 3, modified_ns: 10, file_id: Some(5), sha256: [1; 32] }
}

fn status() -> [UploadStatus<'static>; 1] {
    [UploadStatus { stage: "done", error: "", size: 3 }]
}

fn remote(sha256: [u8; 32]) -> RemoteFile {
    RemoteFile { name: "file".into(), size: 3, is_regular_file: true, sha256 }
}

#[test]
fn same_size_wrong_content_cannot_authorize_deletion() {
    let uploaded = Candidate::new(original(), "file".into()).unwrap()
        .confirm_upload(&status()).unwrap();
    assert!(matches!(uploaded.verify_remote(&remote([2; 32])), Err(GuardError::RemoteNotVerified)));
}

#[test]
fn source_content_change_invalidates_delete_permit() {
    let journal = Candidate::new(original(), "file".into()).unwrap()
        .confirm_upload(&status()).unwrap().verify_remote(&remote([1; 32])).unwrap()
        .record_saved(true).unwrap();
    let changed = Original { sha256: [2; 32], ..original() };
    assert!(matches!(journal.permit_delete(&changed, false), Err(GuardError::OriginalChanged)));
}
