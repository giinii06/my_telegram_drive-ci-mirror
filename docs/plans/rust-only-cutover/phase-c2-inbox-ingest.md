# Phase C.2 — Native MTProto → private Rust inbox (no Go DB writes)

**User flow:** an operator authenticates a dedicated Telegram user session, selects an accessible peer, and invokes `tdrive-mtproto ingest @peer 1..100`. Rust directly reads Telegram history over MTProto and durably writes recognized, bounded TDX1 operation headers to its **own** SQLite inbox. Repeated ingestion must not create duplicate operations. Modified headers must preserve their first-seen canonical copy and record tamper evidence. A separate `inbox-stats` command reads counts without credentials/network. Later native sync consumes inbox through the 17-operation Rust projector; it is **not** equivalent to the full Go projector or the production media/vault pipeline.

## Gate
- Existing target: GitLab project 89276294 Draft MR !24, branch `refactor/rust-only-cutover-20261122` at `33d0eccfa2c6ee3b43fa51eb15ec89a2f2c2bea1`. Prior design-approved full Go→Rust/GPUI/Docker parity still applies; Go remains authoritative until verified cutover.
- Existing C.0 and C.1 are **not** compiled/tested. GitLab pipeline #3098891720 at code SHA `3a9a8656...` is now **failed** with external `github-actions/ci-hub-request=failed`; no actual job logs available. Missing proof is NOT a passing gate nor a user waiver.
- This is an explicit **unverified implementation attempt** within already `in_progress` Phase C, not a phase advancement to `tested/verified/completed`. Do not merge/deploy or change CI provider, mirror, runner, secrets, visibility, user account/data.

## Data shapes and architecture
- Isolated schema `tdrive_native_inbox_v1` in `TDRIVE_INBOX_PATH`, never Go `/data` SQLite DB or `replay_log` tables; no new files in source repository.
- `InboxEvent { peer_key: &str, msg_id: i64, raw_header: &str }` with `UNIQUE(peer_key,msg_id)`; checksum SHA-256 of raw header, accepted op type decoded by existing `tdrive-projection::parse_wire`. `InboxOutcome: Inserted|Seen|Tamper`.
- SQLite `IMMEDIATE` transactions. First header immutable. Different header records latest SHA and count in `inbox_tamper`. Never deserialize/execute untrusted Telegram captions as SQL or filesystem commands.
- `MAX_HEADER=16 KiB`, peer key <=128 ASCII chars, msg id >0, `TDX1` prefix; skip unrelated messages, fail if any apparent TDX1 message is unsupported or invalid; report processed/inserted/seen/tamper/skipped.
- Use a private 0700 directory and 0600 SQLite file (same session file validation pattern). Appends are **not** remote writes. This stage does not schedule retries or write Telegram.
- `ingest` with bound 1–100 messages is intentionally a bounded page, NOT a complete backfill or lossless update subscription. Do not claim it can catch gaps, process edits/deletions in real time or atomically apply the 17 Go operations.

## Test-first acceptance
- Local in-memory SQLite schema test: recognized TDX1 new/duplicate/changed, tamper count/first-seen unchanged, malformed or oversized inputs rejected, wrong schema fails closed and no mutation; sqlite rollback of rejected rows.
- CLI argument negatives: missing/invalid peer, limit 0/101, missing private inbox path; `inbox-stats` offline no Telegram credentials.
- Run exact-SHA Rust fmt/clippy/test/build and Docker `--help`/offline `inbox-stats` on disposable private directory. Then **isolated Telegram test-account** read-only ingest and restart replay, no real account credentials in CI. Verify the checked-out SHA and job logs, not the external bridge status.
- Later C.3: full incremental history difference + cursor/watermarks and all 17-op transactional projection; C.4: vault/TDE1 uploads/downloads with durable idempotency; D: GPUI, API, Docker cutover, rollback; E: remove Go after complete proof.

## Evidence status before edits
No Cargo/rustc in local container; private source checkout and GitHub jobs unavailable. Therefore tests may be authored but RED/GREEN and E2E cannot be claimed. This plan is a foundation with the explicit consuming user flow described above, not finished production.
