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

### C.1 exact-SHA checkpoint — 2026-11-22

**Scope delivered as GitLab source:** standalone nested Rust Cargo workspace `apps/tdrive-mtproto`, not yet integrated into `crates/tdrive-telegram`/Go v16 17-op projector or vault and **not deployed**.

- Plan `3a92e41922acb2d2261b63e3f41276c98f9a148c`; four policy tests `b2e9d3a066790c8074b195cc87a41c63c00fcb2c` and CLI/path policy `172f22b86d4b041813065b24b797e68ab85daebf`; match fix `fc1217a6dedb45146986bc7370a36648abc99730`.
- Actual MTProto Grammers 0.10.0 CLI source `93d14666365c15db4946800e56f55f6ab852e1d4`: interactive phone/code/2FA sign-in, private SQLite session, `status`/dialogs/history, explicit-confirm upload of local regular file, staged no-clobber download. Not Bot API and no Go subprocess. Library APIs checked against grammers docs; no Rust compilation or test execution.
- Ubuntu/macOS native GitHub workflow and separate non-root headless Dockerfile `43f4a1687cd0780e609e627b78ec08e1dfc12487`. No ports or keys in image and no deployment. Vulnerable pinned `rpassword` 7.4.0 updated to 7.5.4 in `e2a607bc1a7fae25346a429a8518b19bd427474c`; operator readme `1997b4a2da08e5763b0c99e715f680261660b075`.
- Extra path refusal regression test `fa50ba4227ee866efd56d8a579aa165c22476f0d`; fail-closed on non-ENOENT destination metadata errors code `3a9a86568a3d61c176d180ece1866c459c0302f2` **current code SHA**. Five policy test cases written, not run.
- Source review of exact GitLab files at `3a9a86568a3d61c176d180ece1866c459c0302f2`: Grammers calls exist, session privacy and explicit upload confirmation wired, Docker non-root no exposed port, Linux/macOS workflow source. Structural source review is not a build.
- GitLab pipeline [#3098891720](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098891720) created at exact code SHA `3a9a86568a3d61c176d180ece1866c459c0302f2`, reported `dispatch_github_actions=success` with zero GitLab jobs. External matching GitHub native Cargo/job evidence is unavailable. Classify `dispatch=success`, `provider_result=unknown`, `jobs_verified=unknown`, `e2e_verified=unknown`. Earlier unrelated bridge [#3098880127](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098880127) eventually returned external `failed`; without logs its cause cannot be attributed to a specific Rust source defect.
- **Unavailable tests:** `cargo +1.92.0 fmt --manifest-path apps/tdrive-mtproto/Cargo.toml --check`, `cargo +1.92.0 clippy --manifest-path apps/tdrive-mtproto/Cargo.toml --all-targets -- -D warnings`, `cargo +1.92.0 test --manifest-path apps/tdrive-mtproto/Cargo.toml`, `cargo +1.92.0 build --manifest-path apps/tdrive-mtproto/Cargo.toml --release`, Docker build/help, live isolated Telegram account login/session restart/history/upload/download and non-Go API integration. No cargo/rustc, no authenticated private checkout, no Telegram test account or GitHub job traces accessible. The nested workspace has **no verified Cargo.lock** and transitive package resolution remains unknown.
- **Two sequential self-review lenses**: [reviews/phase-c1-mtproto.json](reviews/phase-c1-mtproto.json), reviewed code SHA `3a9a86568a3d61c176d180ece1866c459c0302f2`. Requirements review: commands and ports targeted but no user-flow E2E and no existing Rust storage integration. Security review: private session path, no secret argv, no-clobber download; unresolved path/TOCTOU race, session resource lifecycle, memory handling, unsupported network errors/FloodWait, upload idempotency and TLS/prod API. Not independently reviewed.
- Phase C `in_progress` (C.1 **source implemented/unverified**), Phase A/B not verified, Phase D/E not started. Go files remain ~517; no Go deletion, merge, deploy, production auth/data, runner, mirror, visibility or secret modifications.
