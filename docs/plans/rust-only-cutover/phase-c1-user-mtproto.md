# Phase C.1 — Native Rust user-account MTProto and headless execution

**Target:** `blond11qq/my_telegram_drive` / Draft MR !24 / branch `refactor/rust-only-cutover-20261122`, start SHA `f2cda87565f654ce1e803d46c272e861d7fa8c0e`. **Status: planned, unverified**. Existing authorizations: preserve existing Go until full Rust parity; no merge/deploy/account tests with user credentials, no runner/secret/mirror changes. Grilling gate: no new user-owned decisions; previous full Go→Rust plan and macOS+Linux/headless goals already settled.

## Verified third-party API
Use documented `grammers-client 0.10.0`, `grammers-mtsender 0.10.0`, `grammers-session 0.10.0` (SqliteSession default), `tokio`; official grammers source examples show `SenderPool { runner, handle, .. }`, `Client::new(handle)`, `tokio::spawn(runner.run())`, `request_login_code`, `sign_in`, optional `check_password`, `iter_dialogs`, `iter_messages`, `upload_file`, `send_message`, `get_messages_by_id` and media download. No Bot API. Source: https://docs.rs/grammers-client/0.10.0/grammers_client/ and https://docs.rs/crate/grammers-client/latest/source/examples/downloader.rs .

## Architectural decision / dependency separation
Create `apps/tdrive-mtproto/` as a **standalone nested Cargo workspace**, separate from the existing parent Rust workspace/Cargo.lock. Why: its new protocol/runtime dependency graph cannot be legitimately represented in the current root `Cargo.lock` without running Cargo; the current sandbox has no `rustc`/`cargo`. Do not manually invent Cargo.lock hashes or break root `--locked` CI. This is a real grammers-backed executable for a disposable Telegram test account, but **not yet production engine wiring**. Later migration phase will unify workspaces and lockfiles after actual dependency resolution and tests.

## Stage C.1 actor outcomes
1. `status`: only after configured `TG_API_ID`/`TG_API_HASH`, verify authorization against MTProto.
2. `login`: request code for phone, prompt code and optional 2FA, persist authenticated session in private session DB directory. Never put 2FA/code or session secret in argv, CLI logs, GitLab, the web or Docker image.
3. `dialogs` and `history <peer> <limit>`: read via MTProto and bounded iteration, public username or existing private dialog, never assume Bot API or access hash from ID alone.
4. `upload <peer> <absolute-file>`: require explicit per-run confirmation before publishing Telegram content; disallow nonregular/symlink file. `download <peer> <msg-id> <absolute-destination>`: target must not exist, download only to restrictive temporary file and atomic no-clobber rename/hardlink after success. No live production data.
5. Independent headless Docker build and GitHub Actions CI definition for this nested package; **no action deploys, logs in, mutates live Telegram data or changes CI provider**.

## Security / failure acceptance
- Fail closed if API ID/hash missing, invalid session directory path/mode, session symlink or group/world-readable permissions, missing login, unsafe peer or unbounded history count, source symlink, output already exists or failed transfer.
- Never print secrets. Do not save SMS or 2FA codes. Session DB holds Telegram authorization material; mount private `0700` folder.
- Limit history and dialog enumeration, reject ambiguous username/directory IDs, require explicit confirmation for remote upload. No delete/move remote operations.
- Test the pure path/CLI guards locally; native Rust compiler checks and real isolated Telegram account E2E (login, restart, history, upload, download, wrong credentials, flood wait) required **before tested/verified**. External GitLab ci-hub callback is not Rust job evidence.
- A production engine still needs original `tdrive-telegram` ports and v16 SQLite 17-op projection, durable MTProto random IDs, vault encryption, media/WebDAV, API, GPUI and rollback. This standalone path is a C.1 executable vertical slice, **never equivalent to full Go removal**.

## Acceptance evidence
- `cargo test --manifest-path apps/tdrive-mtproto/Cargo.toml`; `cargo fmt --manifest-path ... --check`; `cargo clippy --manifest-path ... --all-targets -- -D warnings`; `cargo build --manifest-path ... --release`.
- Build a Docker image and run `--help` with no secrets; real account login/network tests only with an isolated account and explicit consent. Log actual stdout/status and exact checkout SHA.
- Existing state ledger `docs/plans/rust-only-cutover/phase-state.json` C stays `in_progress`; do not advance other blocked gates automatically.
