# Phase 06 — TDX1 format parity (plain subset only)

Rust's existing parser now has a separate fail-closed formatter for the
seven plain legacy operations: file, meta, mkdir, rename, move, rmdir, tomb.

The exact strings in tests/fixtures/tdx1/formatted.tsv are checked by the
Go production Format(Parse(...)) and the Rust format_simple_wire(Parse(...))
functions. Positive test coverage includes Unicode, QueryEscape reserved
characters, empty root-parent, nested folders, size and timestamp fields.

No writable CAS, sealed filenames, multipart, renditions, encryption config,
or Telegram send operation uses this formatter. Advanced operations explicitly
fail with UnsupportedFormat; it is not a complete TDX1 serializer.

Phase 06's full projection, replay_log / SQLite transactions, live sync, name
normalization and encrypted metadata preservation remain outstanding.
