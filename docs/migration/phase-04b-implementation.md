# Phase 04.b — Go/Rust directory-listing parity (increment)

Goal: on a detached Go v16 SQLite snapshot, read folders and files by (channel_id, parent_id) with exactly the two original SELECT statements from backend/projection/read.go::ListFolderContentsContext.

Go oracle: backend/projection/rust_sqlite_fixture_test.go creates the actual Go schema and a populated **synthetic** database. It then calls the actual Go ListFolderContents for root, nested folder, tombstoned folder, missing folder and second channel. Rows contain encryption metadata, a logical file whose content msg differs, Unicode displayed name, a tombstoned file/folder and deterministic ordering. A protected synthetic-only TSV oracle is passed to Rust in CI.

Rust: crates/tdrive-storage/src/listing.rs exposes list_folder_contents_snapshot, FolderListing, FolderSlim and FileSlim. The function requires detached regular non-symlink SQLite (no WAL sidecars), SQLite integrity and Go v16. The caller must provide the snapshot path explicitly; no database is created, modified or migrated. Data sorting/filtering uses Go's SQL, not custom Rust sorting.

Test: crates/tdrive-storage/tests/go_sqlite_fixture.rs reads the Go fixture and Go-produced listing oracle in the same pipeline; compares exact rendered rows and DB file bytes before/after. CI fails if the Go contract diverges.

Limits: deliberately read-only and detached; not a live API, replay transaction engine, encryption key holder or production data migration. Operational Go remains sole writer. Phase 04.c–h and all later dependent features are pending.
