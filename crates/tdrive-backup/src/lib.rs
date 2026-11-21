//! Offline backup safety gate. Never performs filesystem or Telegram I/O.
//! The Go backup agent remains the only running agent until integration tests.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    InvalidCandidate,
    UploadNotComplete,
    SizeMismatch,
    RemoteNotVerified,
    JournalNotSaved,
    OriginalChanged,
    MissingFileIdentity,
    KeepOriginal,
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "backup deletion safety gate denied: {self:?}")
    }
}
impl Error for GuardError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Original {
    pub size: i64,
    pub modified_ns: i64,
    /// Stable per-filesystem object identifier captured from trusted metadata.
    /// None is never acceptable when authorizing deletion.
    pub file_id: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub name: String,
    pub size: i64,
    pub is_regular_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadStatus<'a> {
    pub stage: &'a str,
    pub error: &'a str,
    pub size: i64,
}

/// Exact Go cmd/tdrive-backup/client.go summarizeUpload verdict semantics.
pub fn summarize_upload(files: &[UploadStatus<'_>]) -> (bool, String) {
    if files.is_empty() {
        return (false, "upload job reported no files".to_owned());
    }
    let mut done = true;
    for file in files {
        match file.stage {
            "done" => {}
            "error" => {
                return (
                    false,
                    if file.error.is_empty() {
                        "server reported an upload error".to_owned()
                    } else {
                        file.error.to_owned()
                    },
                );
            }
            _ => done = false,
        }
    }
    (done, String::new())
}

pub struct Candidate {
    original: Original,
    expected_remote_name: String,
}
pub struct Uploaded {
    original: Original,
    expected_remote_name: String,
}
pub struct RemoteVerified {
    original: Original,
}
pub struct JournalSaved {
    original: Original,
}
pub struct DeletePermit {
    original: Original,
}

impl Candidate {
    pub fn new(original: Original, remote_name: String) -> Result<Self, GuardError> {
        if original.size < 0
            || remote_name.is_empty()
            || remote_name == "."
            || remote_name == ".."
            || remote_name.contains('/')
            || remote_name.contains('\\')
            || remote_name.contains('\0')
        {
            return Err(GuardError::InvalidCandidate);
        }
        Ok(Self {
            original,
            expected_remote_name: remote_name,
        })
    }

    pub fn confirm_upload(self, status: &[UploadStatus<'_>]) -> Result<Uploaded, GuardError> {
        let (complete, failure) = summarize_upload(status);
        if !complete || !failure.is_empty() || status.len() != 1 {
            return Err(GuardError::UploadNotComplete);
        }
        if status[0].size != self.original.size {
            return Err(GuardError::SizeMismatch);
        }
        Ok(Uploaded {
            original: self.original,
            expected_remote_name: self.expected_remote_name,
        })
    }
}

impl Uploaded {
    /// The caller must have listed the authenticated destination parent.
    pub fn verify_remote(self, entry: &RemoteFile) -> Result<RemoteVerified, GuardError> {
        if !entry.is_regular_file
            || entry.name != self.expected_remote_name
            || entry.size != self.original.size
        {
            return Err(GuardError::RemoteNotVerified);
        }
        Ok(RemoteVerified {
            original: self.original,
        })
    }
}

impl RemoteVerified {
    /// Only call after the real journal adapter completes its durable write.
    pub fn record_saved(self, persisted: bool) -> Result<JournalSaved, GuardError> {
        if !persisted {
            return Err(GuardError::JournalNotSaved);
        }
        Ok(JournalSaved {
            original: self.original,
        })
    }
}

impl JournalSaved {
    pub fn permit_delete(
        self,
        observed_now: &Original,
        keep_original: bool,
    ) -> Result<DeletePermit, GuardError> {
        if keep_original {
            return Err(GuardError::KeepOriginal);
        }
        if self.original.file_id.is_none() || observed_now.file_id.is_none() {
            return Err(GuardError::MissingFileIdentity);
        }
        if &self.original != observed_now {
            return Err(GuardError::OriginalChanged);
        }
        Ok(DeletePermit {
            original: self.original,
        })
    }
}

impl DeletePermit {
    /// This is evidence only, not a safe filesystem unlink operation.
    /// Any real delete must re-verify identity while guarding TOCTOU.
    pub fn protected_original(&self) -> &Original {
        &self.original
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original() -> Original {
        Original {
            size: 12,
            modified_ns: 1_700_000_000_000,
            file_id: Some(12345),
        }
    }

    fn status() -> [UploadStatus<'static>; 1] {
        [UploadStatus {
            stage: "done",
            error: "",
            size: 12,
        }]
    }

    fn candidate() -> Candidate {
        Candidate::new(original(), "archive.txt".to_owned()).unwrap()
    }

    fn remote() -> RemoteFile {
        RemoteFile {
            name: "archive.txt".to_owned(),
            size: 12,
            is_regular_file: true,
        }
    }

    fn journaled() -> JournalSaved {
        candidate()
            .confirm_upload(&status())
            .unwrap()
            .verify_remote(&remote())
            .unwrap()
            .record_saved(true)
            .unwrap()
    }

    #[test]
    fn complete_remote_journal_and_stable_source_are_all_required() {
        assert_eq!(
            journaled().permit_delete(&original(), false).unwrap().protected_original(),
            &original()
        );
        assert!(matches!(
            candidate().confirm_upload(&[UploadStatus {
                stage: "uploading",
                error: "",
                size: 12
            }]),
            Err(GuardError::UploadNotComplete)
        ));
        assert!(matches!(
            candidate().confirm_upload(&[UploadStatus {
                stage: "done",
                error: "",
                size: 11
            }]),
            Err(GuardError::SizeMismatch)
        ));
        assert!(matches!(
            candidate()
                .confirm_upload(&status())
                .unwrap()
                .verify_remote(&RemoteFile { size: 11, ..remote() }),
            Err(GuardError::RemoteNotVerified)
        ));
        assert!(matches!(
            candidate()
                .confirm_upload(&status())
                .unwrap()
                .verify_remote(&remote())
                .unwrap()
                .record_saved(false),
            Err(GuardError::JournalNotSaved)
        ));
    }

    #[test]
    fn never_authorize_after_file_swap_or_keep_original() {
        let mut modified = original();
        modified.file_id = Some(98765);
        assert!(matches!(
            journaled().permit_delete(&modified, false),
            Err(GuardError::OriginalChanged)
        ));
        modified.file_id = None;
        assert!(matches!(
            journaled().permit_delete(&modified, false),
            Err(GuardError::MissingFileIdentity)
        ));
        assert!(matches!(
            journaled().permit_delete(&original(), true),
            Err(GuardError::KeepOriginal)
        ));
        modified = original();
        modified.modified_ns += 1;
        assert!(matches!(
            journaled().permit_delete(&modified, false),
            Err(GuardError::OriginalChanged)
        ));
    }
}
