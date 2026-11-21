# Phase 08 — backup deletion safety: initial Rust increment

The Go backup agent under cmd/tdrive-backup is still the production agent.
Rust tdrive-backup-core is pure in-memory decision logic and does NOT upload,
touch the journal, scan a user's disk, or delete any file.

The type-state path is Candidate → Uploaded → RemoteVerified →
JournalSaved → DeletePermit. A permit requires a single fully completed
upload job with exactly matching bytes, an authenticated destination listing
showing the exact regular file and size, durable journal persistence, and an
unchanged source snapshot with a stable file identity. KeepOriginal blocks it.

Unknown file identity, changed size/mtime/inode, failed or partial upload,
wrong-size remote, directory-only response, or failed journal persistence all
fail closed. These tests cover the source-swap and unexpected remote state.

The pure state gate is NOT a safe unlink primitive. A future platform-specific
adapter must recheck identity at the actual unlink boundary while defending
against symlinks and TOCTOU races. The Rust crate exposes NO file deletion API.

11 shared status-fixture cases drive the Go production summarizeUpload helper
and Rust summary implementation. This detects future Go/Rust drift on the
backup HTTP upload status contract.

Still outstanding: HTTP backup client, authenticated destination listing,
Go journal JSON compatibility, durable fsync/atomic persistence, file-descriptor
safe deletion, real backup daemon, restart/fault-injection E2E and GPUI.
Do not interpret this increment as full Phase 08 completion.
