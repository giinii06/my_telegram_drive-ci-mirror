# Design — TDrive Rust Web-only cutover

## Decisions

| Area | Decision |
|---|---|
| UI | Browser web UI only; retain Svelte/Bun |
| Android | Remove APK project/build/support docs; do not port |
| Desktop | No GPUI, native app, Electron or WebView shell |
| Server | Headless Rust server; Docker/Compose + Tailscale primary |
| CLI | Operator/automation only |
| Protocols | Preserve HTTP/WebDAV contracts |
| Go removal | Only after complete parity and exact-SHA evidence |
| Production | No migration, merge or deployment in this task |

## Boundaries and data flow

Browser request → authenticated Rust HTTP API → typed application service → storage/projection/crypto/Telegram/media/WebDAV. API does not issue SQL or raw Telegram calls. One server process owns SQLite writer and Telegram session lifecycle. CLI communicates over an owner-only local control channel or authenticated loopback API after the IPC/API ADR. Docker has no GUI or display dependencies.

- Browser UI remains responsive for desktop and mobile viewports.
- Existing Go HTTP route contract is the compatibility oracle: method, query, body, cookie/header, status, error JSON, streaming and timeout behavior.
- Storage and projection own durable transaction/replay order; Telegram adapter owns MTProto session, flood waits and ambiguous send reconciliation.
- Crypto wraps streams before upload; media range readers decrypt only authorized ranges.
- Progress/events use reconnect-safe cursors and never include secrets.

## Failure/recovery

Single-writer lock and SQLite transactions are mandatory. Unknown Telegram send outcome is reconciled via stable random_id/history before retry. Migration/rebuild runs on consistent clones. Locked vault, invalid key, tampered ciphertext and uncertain permissions fail closed. Backup source deletion requires server completion receipt plus independent SHA-256 and byte-count verification. SIGTERM drains work, persists cursors/journal, closes DB and releases lock.

## Explicit exclusions

GPUI/native desktop client; Android APK/WebView app; GUI-specific packaging; frontend rewrite; immediate Go deletion; release asset/user-data/Telegram-message deletion. Headless macOS/Windows server packages may remain if tested.

## Deferred inventory gate

WebDAV wire compatibility remains required. Whether OS-specific FUSE/GVfs/WinFsp adapters must be ported is decided only after Phase 01 maps real existing user flows. Do not remove a Go feature solely because it lacks a GPUI screen.
