# Phase 03 — Rust operator encrypted Telegram roundtrip (future integration owner)

**Status:** not_started
**Implemented:** no
**Tested:** no
**Committed:** no
**Gate version:** 2.2
**Outcome kind:** user_flow
**Entrypoint:** real Rust client CLI/desktop or headless-server public operator flow
**Dependencies:** Phase 01/02 CI validation, Phase 07.c Vault, Phase 05 MTProto, Phase 06/08 projection, media and server wiring

## User-visible positive acceptance

On an **isolated** owner-authorized Telegram test account and disposable drive, user logs into the Rust client, creates a Vault and uploads an encrypted file; after restart, a second process authenticates and downloads it, verifies decrypted SHA-256, filename, metadata and permissions, without the Go backend. Repeat on supported macOS and Linux Docker, with explicit Windows fallback policy.

## Negative acceptance

Wrong password and tampered ciphertext never disclose or publish partial plaintext; canceled/resumed upload never creates an inconsistent record, server crash/replay preserves the database and original file, unknown message / rate limit follows Go retry semantics, credentials never enter CI log, unauthorized users cannot list or download.

## Implementation boundary

This Phase is a **future dependency owner only** and is NOT started by the CI-recovery scope. Do not request personal credentials or grant merge/deploy permissions. A green Python YAML checker, Rust unit tests or GitHub provider callback is not Phase 03 evidence. Reference the canonical [complete migration phase ledger](../../migration/phase-ledger.md) for Phase 05–15 functionality and rollback.
