# Phase 04.d — Go v16 SQLite all-core-table row fingerprints

Goal: compare actual stored values, not just table counts or SQLite schema existence.

The Go fixture generator and Rust snapshot reader independently traverse all 14 REQUIRED_TABLES, and for every column value use SQLite typeof plus hex(CAST(value AS BLOB)). This retains BLOBs, unicode, embedded NULs, SQL NULL and mixed integer/text representations. Column names and table names are fed to SHA-256 and every field has a deterministic unsigned 64-bit big-endian length prefix. Rows are ordered by all selected expressions, never by arbitrary rowid or insertion order.

Go generates a synthetic SQLite database using production Go schema and exports per-table name, row count and SHA-256; Rust uses the same detached file and must output matching values. No source data, passwords, encryption keys, captions or filenames are printed by the digest reporter.

Rust limits each table to 1,000,000 rows and 32MiB of serialized digest input and rejects an oversized snapshot. This bounds the accepted workload, though SQLite may allocate temporary query buffers; this is not a safe arbitrary untrusted live-db service. It requires an externally created consistent SQLite backup without WAL/SHM sidecars and never writes a database.

Excluded: supported legacy-schema migration, live DB write locks and transactions, WAL-aware snapshot creation, Go→Rust migration dry-run, encrypted vault operations, actual Telegram upload/replay and production cutover. This is only a verifiable read-only Phase 04.d increment. Matching HEAD CI is required before status can change to verified.
