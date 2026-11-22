# Rust-only migration and conditional Go removal

## Approved contract
- GitLab source of truth: `blond11qq/my_telegram_drive` (project 89276294). Start from previous cumulative Draft MR !23 HEAD `17df07d389fe317b645b4f9ef9bf5bc0d1fe857c`; work branch `refactor/rust-only-cutover-20261122` targeting `feat/rust-offline-tde1-cli-20261122`.
- User's latest instruction supersedes the old *design-time* prohibition on deleting Go source: replace **all** Go backend code with Rust and remove Go sources, Go module/build dependencies and Go CI **only after** each existing Go-owned user flow has a Rust replacement. The source remains private. No merge, deployment, force push, production account/data edits, mirror/secret/runner setting edits.
- Required product: MTProto signed-in user session, vault/crypto, SQLite full mutation/replay, sync, file transfer/backup, media/HLS/WebDAV, local daemon/HTTP/CLI, native GPUI desktop, headless Docker, installers/platform parity. Preserve data and recovery.
- Existing [full-parity ledger](../../migration/phase-ledger.md) and [migration master](../my-telegram-drive-rust-migration/README.md) own 00–15 subphases. Those remain the complete implementation scope, not a new shorter substitute.

## Evidence at start
GitLab tree on `17df07d389fe317b645b4f9ef9bf5bc0d1fe857c`: **517** tracked `.go` files and **35** tracked `.rs` files, including tests (numbers not functionality percentages). The only Rust executable is `tdrive-inspect`, an offline inspector/crypto diagnostic. The authoritative Go main remains operational. Rust MTProto user sessions and sync writer do not exist. Running Go deletion **now** would intentionally break the application and invalidates the requested functionality. Treat that as a failed migration, not a completed Go removal.

**CI:** effective project `ci_config_path=.gitlab/ci-hub-request.yml@blond11qq/ci-hub-smoke`, not local `.gitlab-ci.yml`. GitLab pipeline 3098837254 on `970dafcc04a7f1e008bb5117ac5aff72b4a9ec7e` returns `dispatch_github_actions` and `github-actions/ci-hub-request` success but no actual six-job Rust build logs. This does not verify any Cargo command. Container has Go, git but **no Rust toolchain** and cannot resolve private GitLab DNS; GitLab connector can write tracked source but cannot perform local Cargo build or access GitHub Actions job details. No waiver has been approved.

## Current execution sequence
1. [Phase A](phase-a-native-read-and-ci.md), executable Rust read-only folder browsing from detached Go v16 SQLite, with synthetic SQLite fixtures and Rust-only (no Go commands) smoke workflow. This is a **user-flow diagnostic increment** consumed later by the Rust API/GUI, not a replacement of Go sync/backend. Positive, invalid input, no-writes; separately test CLI output.
2. Phase B: direct Rust DB single-writer and full 17-op replay/sync parity, with explicit transaction/rollback and Go DB fixtures. Must retain Go reference/oracles until equivalent cases are verified.
3. Phase C: Rust real user-account MTProto auth/upload/download, encryption vault lifecycle, file recovery, media/HLS/WebDAV and daemon APIs.
4. Phase D: GPUI and headless Docker, cross-platform build and representative end-to-end test.
5. [Phase E](phase-e-remove-go.md): delete Go production sources, go.mod/go.sum and Go-specific workflows/install steps only **after** replacement checklist and exact SHA job evidence; build/test a clean Go-free checkout. Keep migration comparison fixtures in language-neutral format as needed; never remove a working backend before replacement.

## CI and phase-state gates
For each code change: test-first behavior + RED when runnable; implementation; Cargo fmt/clippy/build/test and backend integration on exact code SHA; original Go parity while it exists; two-lens review with actual source SHA; append checkpoint; Draft MR. A generic bridge callback is `provider_result` at most, not `jobs_verified`. No production cutover without separate authorization. This plan cannot make the entire 517-Go-file conversion fit into a single unverified code commit.

## Work checkpoint at 12a39ccd

Phase A Rust CLI + Go-free Rust CI source has been committed, but **not executed** by cargo. Python Go-deletion audit 5/5 tests are locally verified with exact GitLab file hashes; YAML structure parsed. GitLab #3098859551 only confirms dispatch. Go removal Phase E remains **blocked** with 517 Go files and missing Rust parity. See [Phase A evidence](phase-a-native-read-and-ci.md), the [phase state](phase-state.json), and [two-lens self-review](reviews/phase-a.json). This is **not** the completed Rust migration.

## Rust replay journal code checkpoint (0571e052)

Phase B.0 implementation is a transaction-safe replay metadata increment and **not** the 17-op projector or live Telegram sync. Source SHA `0571e0527231d9b0c04292b148cd49228f8e1f66`, GitLab dispatch #3098872773, Rust compilation/job verification unknown. Details and two-lens self-review under [phase B.0](phase-b0-replay-journal.md) and [review](reviews/phase-b0-replay.json). Phase B remains in progress, not tested/verified; full Go deletion blocked.

## Local Rust HTTP API checkpoint (6756046e)

C.0 source code adds authenticated loopback-only read-only HTTP to the existing offline SQLite inspector, plus 5 parser tests and 1 socket integration test **not executed**. Code SHA `6756046e3d7a6fdefb7c458f9f859ab26778ec13`, matching GitLab dispatch #3098880127. This is **not** a Telegram user login/server/media backend or Go-parity completion. See [C.0 evidence](phase-c0-readonly-http.md), the [review](reviews/phase-c0-http.json) and [phase ledger](phase-state.json). No Go code deleted.

## C.1 standalone Rust MTProto checkpoint (3a9a8656)

Native Grammers 0.10.0 user CLI + private session, interactive login/2FA, dialogs/history, explicit-confirm upload, no-clobber download, Ubuntu/macOS workflow and separate headless Dockerfile were committed. **Not compiled/tested**, no real user-account E2E or 17-op/vault/Go integration. Matching GitLab CI #3098891720 only proves dispatch. See [C.1 plan and evidence](phase-c1-user-mtproto.md), [self-review](reviews/phase-c1-mtproto.json), and [state ledger](phase-state.json). No Go deletion.

## Native MTProto inbox / Docker checkpoint (`c9bc5bb6`)

Bounded read-only Telegram history ingestion into a separate private Rust SQLite inbox, idempotent/tamper records, credential-free offline stats, isolated nonroot Compose and repaired Docker path-crate context. Rust execution **unverified**; GitLab #3098911766 is dispatch-only. See [C.2 phase evidence](phase-c2-inbox-ingest.md), [C.2 self-review](reviews/phase-c2-inbox.json). Full 17-op projection, true incremental sync, vault/GPUI/headless server and Go deletion remain incomplete.

## Native Telegram crash-safe catchup checkpoint (e2e0ecc3)

Added persisted per-peer SQLite watermark, history interval scanning with a 10,000-message cap, strict descending order, stale cursor conflict rejection, and fail-closed no-cursor-advance on partial/transport errors. **Code exists but no cargo run/Telegram E2E**. GitLab #3098971908 failed; child GitLab ci-hub-smoke #3098971934 is skipped/manual, not Rust jobs. See [C.3 evidence](phase-c3-history-catchup.md) and [self-review](reviews/phase-c3-catchup.json). All Go parity/GPUI/Docker completion gates still blocked.

## Native Rust supervised watch checkpoint (3c7c2956)

Added experimental MTProto `watch` command with SIGINT/SIGTERM, private opt-in Docker Compose, Linux/macOS CI contract; source SHA `3c7c2956a40ce65b0b73de58f23531a6fa495f05`. **Not compiled or validated in a running Telegram/Docker environment.** Static source review 15/15 only. [GitLab CI #3099037733](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3099037733) failed after dispatch; child CI skipped. See [C.4 phase](phase-c4-headless-watch.md), [self-reviews](reviews/phase-c4-watch.json) and [phase state](phase-state.json). Full Go parity/GPUI/headless drive remains blocked.

## CI publication scan incident (2026-11-22)

Exact provider failure text is **`The requested Git history did not pass the publication scan`**, not a proven Rust compilation failure. GitLab #3099037733 dispatched successfully but provider failed; ci-hub-smoke child #3099037748 was skipped with an echo-only manual job. No actual Cargo job logs available. Public-history scanner must not be bypassed. See [CI root-cause evidence](ci-publication-scan-blocker.md); phase status remains unverified, Go is preserved.
