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

## Implementation checkpoint — Go v16 transactional replay append (2026-11-22)

- Plan SHA: `99dbaac13fad641ef5e8d1333d556ccb87ec25fb`.
- Test-only SHAs: `a91038f7f5d4b33971fa58f688dd79588220ee11` (five synthetic SQLite tests) and `3d1c5a62e8041383b85749b4c67a466baa2c2dd0` (reject mismatched/absent JSON Type). **No executed Rust RED**.
- Code SHAs: `5ccd6291f74815cad775ccb6491e4e295cbd77e2` adds `crates/tdrive-storage/src/replay_writer.rs` and existing Rust projection dependency; `ee27100817e5048b3474b6aa6ab0b0430883120a` enforces JSON Type match; `0571e0527231d9b0c04292b148cd49228f8e1f66` normalizes missing JSON Type to explicit invalid payload. **Exact latest code SHA:** `0571e0527231d9b0c04292b148cd49228f8e1f66`.
- Function `tdrive_storage::append_replay_event` accepts only caller-owned `&mut rusqlite::Connection` and validated event metadata. Requires v16 schema version, parseable TDX1 known type, 1 MiB JSON object with consistent `Type`, bounded header, positive channel/message/seen_at and nonnegative actor. `BEGIN IMMEDIATE`/commit; first-seen `replay_log` remains immutable; duplicates and tamper tracked by SHA-256. No vault key or Telegram logins exposed.
- Evidence actually executed: a **small independent Python sqlite3 SQL smoke** in the local container checked JSON validation including a mismatched/missing Type and exercised the same `replay_log` insertion and tamper update; first header remained unchanged, one tamper record existed. This is **not a Rust code compile or compiled integration test**, and it does not prove Go-Rust replay parity.
- CI: [GitLab pipeline #3098872773](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098872773) created on exact SHA `0571e0527231d9b0c04292b148cd49228f8e1f66`, with `dispatch_github_actions=success`; no GitLab job traces or actual GitHub six-job Rust source SHA/log evidence. `dispatch=success`, `provider_result=unknown`, `jobs_verified=unknown`, `e2e_verified=unknown`.
- Required commands **not run**: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test -p tdrive-storage --locked`, compiled SQLite write integration, cross-platform E2E. Available container has no `cargo` or `rustc`, private GitLab clone DNS unreachable.
- Review: [reviews/phase-b0-replay.json](reviews/phase-b0-replay.json) records sequential requirements/security **self-review**, not independent validation. Major blockers: Go's `ProjectFromOp` applies journal and all 17 projection operations in the same transaction, whereas this Rust increment writes only journal metadata. Do **not** call it a full sync engine or use on production databases. Caller must acquire the exclusive backend lock, restore a disposable backup clone, and supply its writable connection. The function itself does not enforce that external precondition.
- Stage B `in_progress` for this **unverified** subphase; its full completion/Go cutover cannot be claimed, nor can previous Phase A be advanced without real job proof. Telegram user MTProto auth/upload/download, whole DB writer, server, GPUI, Docker and Go removal remain incomplete. No deletion, merge/deployment, CI provider/mirror/runner/secret/visibility change, live user data or real account access.
