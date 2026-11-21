# Phase 04.g — WAL-consistent SQLite offline snapshot + SHA-256 manifest

Implementation increment: crates/tdrive-storage/src/backup.rs uses rusqlite 0.43.0's SQLite online Backup API. A source may have active WAL and SHM sidecars. Unlike raw cp, the database connection sees committed WAL transactions and Backup::run_to_completion copies them into a separate, self-contained SQLite file.

Safety invariants:
- source is an existing regular non-symlink SQLite v16 database; it is opened read-only with query_only ON, with PRAGMA integrity/schema/foreign key validation
- destination lives in a private 0700 Unix directory, and both snapshot and .manifest must not already exist
- copy is staged in a mode 0600 newly created temp file; Go schema and integrity are inspected before publish; final output is no-clobber linked in the same filesystem
- SHA-256 is calculated over the final SQLite bytes, with manifest recording hash, size, schema version and 14 core table counts only; no decrypted names or user values printed
- directory sync follows snapshot publication and manifest publication; input Go data is never modified or deleted by this function
- tests use a detached Go-generated synthetic v16 database, put the source into WAL mode, commit user_version=73, leave the writer connection open, and verify the backup contains that committed WAL header update
- negative tests assert refusal to overwrite a snapshot or manifest, refusal of source symlinks and refusal of nonprivate destination folders

**Known gaps**: This does not stop a live Go writer, guarantee a Go/Rust cross-namespace writer lock, implement a restore command, create a verified pre-migration rollback clone, cover Windows ACLs, or provide crash atomicity of the snapshot+manifest as a single pair. If manifest publication fails, the new snapshot may remain without its manifest and must be treated as incomplete. No live customer database is tested or accessed. Phase 04.g is not complete until restore drills and migration dry-run are independently verified.

External API reference: https://docs.rs/rusqlite/0.43.0/rusqlite/backup/index.html
