# Phase C.4 — Persistent, supervised Rust MTProto catch-up service (not full Go parity)

Target: GitLab project 89276294, Draft MR !24; baseline SHA af25da8fe7b04672fa6ea4d964226afce01113b7. Earlier complete Go-to-Rust/GPUI/Docker scope remains authoritative. No merge, deployment, live account, secret, user data, mirror or runner changes. Go continues as production backend.

## User-facing outcome
An authenticated user can run `tdrive-mtproto watch PEER BUDGET POLL_SECONDS` in a foreground terminal / experimental private container. It immediately performs existing persisted-cursor `catchup`, repeats at fixed interval, and exits nonzero on Telegram error, invalid TDX1 or scan-budget exhaustion (no falsely advanced cursor). Restart reuses same private SQLite checkpoint and first-seen events. SIGINT and SIGTERM stop gracefully between ticks. No public network ports or remote mutations.

## API / data / controls
- CLI `CliAction::Watch { peer: String, budget: usize, poll_seconds: u64 }`, shared strict peer policy, 1..=10000 budget, 15..=3600 seconds. Wrong input fails before credential loading or Telegram calls.
- Use already-authenticated user MTProto session and `catchup` method, without Go SQLite access. Every successful complete scan advances only its own private inbox per-peer watermark; failed scan exits and supervisor restart can retry safely.
- Tokio signal feature only for exit. One-shot CLI and Docker Compose unchanged; new opt-in rust-watch compose has no exposed port, non-root identity, private bind mount, read-only root, drop caps, no-new-privileges, `restart: on-failure`, no secrets baked into image. Work must not merge/deploy by itself.
- Important: this service only copies Telegram TDX1 metadata to separate inbox. It does not apply complete Go 17-ops, catch edited/deleted older posts via getDifference, manage vault keys, provide HLS/WebDAV, GPUI, HTTP write API or full backup/migration.

## Test-first and finish criteria
- Unit tests: valid/invalid watch args, no help/credentials leak, reject zero/overlarge budgets and too-frequent/overlong poll; no actual network needed.
- Runtime: actual Rust fmt/clippy/test/build on Linux/macOS, Docker build/offline help, with test-user account login, restart+cursor replay, bad-TDX1 and FloodWait exit, SIGTERM behavior, negative parity and no Go DB writes. Capture actual checked-out SHA and job traces, not only GitLab bridge callback.
- No cargo/rustc/Docker/private checkout in local executor; exact-source GitLab CI bridge previously finalized failed with skipped/manual downstream child and no Cargo logs. No permission to call unexecuted tests passing or to claim full Go parity. Phase C remains in_progress until verification.
