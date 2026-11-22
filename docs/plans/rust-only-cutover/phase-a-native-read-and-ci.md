# Phase A — Rust native read-only folder CLI + independent Rust-only CI

**Kind:** user_flow / foundation for API & GPUI. **Status:** not_started. **Gate version:** 2.2.

## Entry point
`tdrive-inspect --list-folder <detached-v16-snapshot.sqlite> <positive-channel-id> <parent-id>` reads a previously validated, offline detached Go v16 SQLite snapshot by using `tdrive_storage::list_folder_contents_snapshot`, prints a single JSON object of ordered folder/file records, preserves all precise i64 identifiers as decimal **strings**, exits nonzero on invalid/unsafe snapshot or channel; never mutates SQLite or accesses Telegram/Go daemon. Root parent is empty string.

## Acceptance
- Correct folders/files with escaping of `"\n\t\\` and Korean/Unicode names; stable original Go query order, string-preserved 64-bit IDs.
- Invalid/non-positive/overflowed channel, path missing, source symlink, active WAL sidecar, invalid schema, corrupt DB -> nonzero, no output JSON and no database creation/changes.
- Positive synthetic v16 SQLite fixture and separate CLI integration test run without Go build or daemon. The synthetic schema is a **test fixture** and cannot establish all legacy Go data parity.
- Rust-only CI workflow with exact checked-out SHA, `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build -p tdrive-inspect`. **Do not claim workflow ran** until actual GitHub jobs confirm. Retain existing Go fixture workflow while parity incomplete.
- No new serializer libraries or changed Cargo.lock unless necessary. No direct/raw secret in stdout.

## Implementation
Add a `--list-folder` command to `apps/tdrive-inspect` using a pure Rust formatter module that correctly escapes JSON. Add unit tests and integration fixture using pre-existing `rusqlite` version as a dev-dependency. Add a separate Go-free GitHub workflow that only checks Rust; keep existing CI wiring/visibility untouched. Zero production data impact.

## Blockers
No local cargo available in runtime; GitHub remote Rust job details not yet observed. Phases cannot be called verified/completed without successful compiled CLI and actual tests.
