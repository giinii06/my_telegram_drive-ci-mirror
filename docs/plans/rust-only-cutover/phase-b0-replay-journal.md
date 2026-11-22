# Phase B.0 — Go-v16 compatible transactional Rust replay journal

**Status:** planned on branch `refactor/rust-only-cutover-20261122`, stacked Draft MR !24. **User-facing consumer:** future Rust Telegram history/difference sync engine feeding the Rust SQLite projection. This is not user-account MTProto connectivity, all 17 Go op application, or live backend cutover.

## Existing authority
- Go `backend/projection/log.go` and `backend/projection/apply.go`: one transaction for replay_log insertion + projector mutation; `(channel_id,msg_id)` first-seen immutable, SHA-256 raw_header conflict tracked in `replay_log_tamper`, deterministic idempotency.
- Go v16 schema `backend/projection/schema.go`; Rust has read-only `tdrive-storage` and `tdrive-projection::parse_wire` (17 op type grammar, incomplete op payload coverage).
- This incremental Rust writer **only** performs the replay journal transaction and tamper detection in a provided SQLite connection. It does not apply op state to folders/files; it must never be wired as a complete sync projector until next phase.

## API and failure conditions
`append_replay_event(connection: &mut rusqlite::Connection, incoming: &IncomingReplayEvent) -> Result<ReplayWriteOutcome, ReplayWriteError>`.
- Explicitly reject bad channel/message, invalid operator-provided timestamp, excessive raw header or JSON payload, unknown/mismatched wire op type and malformed JSON before any insert.
- Require existing Go schema v16; never create or migrate tables; transaction `IMMEDIATE` serializes competing writers.
- Insert immutable first-seen `replay_log` row with Go compatible SHA-256; same SHA again is `AlreadySeen`. Different raw header for existing `(channel,msg)` becomes tamper entry, without rewriting first-seen canonical replay. Use supplied deterministic `detected_at`, no sensitive header content in errors.
- Fail closed on database errors; rollback must preserve all earlier rows and never create schema. Caller is responsible for acquiring Go-compatible `BackendLock` and ensuring an isolated restored clone or, **after** full parity, controlling the single production writer. This API must not open file paths or live Go DB on its own.

## Tests and verification
- In-memory SQLite Go-compatible replay journal schema and schema_version 16.
- insert / repeated same header / changed header detection, idempotency and original record preservation.
- incorrect schema, unsupported TDX1, mismatched op, malformed JSON, invalid IDs and oversize inputs reject without any rows.
- missing tamper table forces SQL rollback; demonstrate transaction remains reusable.
- Actual Rust command: `cargo +1.92.0 test -p tdrive-storage --locked`; Rust fmt/clippy/workspace tests; byte-for-byte Go oracle golden comparison later. No real production account/data.
- The local execution environment currently lacks cargo/rustc and private checkout; RED or GREEN tests cannot be asserted. GitLab pipeline provider callbacks alone are not job proof. Phase B stays implemented/unverified at most and **Go source removal remains blocked**.
