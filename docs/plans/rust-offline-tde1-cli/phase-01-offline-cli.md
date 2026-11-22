# Phase 01 — offline CLI wiring into atomic TDE1

**Kind:** user_flow (offline diagnostic), **Status:** not_started, **gate_version:** 2.2.

## Actor and entrypoint
A migration operator invokes `tdrive-inspect --encrypt-file SOURCE DESTINATION PRIVATE_KEY_FILE` or `--decrypt-file` on Unix (macOS/Linux), without modifying a live Telegram account. The key file contains exactly 32 raw bytes; no password-to-key derivation, no automatic key creation. Output directory must already be private (0700), source must be a regular non-symlink file at an absolute path, and destination must not exist.

## Test-first acceptance
- Positive: 0 bytes / 64KiB boundary / >64KiB roundtrip, decrypt exactly original, keep source and key intact; final file private.
- Negative: wrong key, tampered last AEAD tag, existing destination, insecure or symlinked key file, short/long key, relative paths, untrusted source path. No target publication and no plaintext from failed decrypt. No filesystem or remote data deletion.
- CLI: success exit 0, failure nonzero, generic error without secret bytes; existing inspect options unchanged.
- Rust tests must actually compile/run; `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`. Separate macOS runtime E2E required.
- Read/test private file guard: file exactly 32 bytes, regular and non-symlink, 0600 or more restrictive, no key in argv or stdout.
- Review requirements and security separately; record exact code SHA and CI evidence, not just external provider callback.

## Implementation boundary
Add only `tdrive-crypto` dependency to `apps/tdrive-inspect` and an isolated `offline_crypto.rs` adapter. Reuse existing `encrypt_file_atomic`/`decrypt_file_atomic`, never duplicate cryptography or publish unauthenticated chunks. Keep explicit `#[cfg(unix)]`, do not invent Windows availability.

## Evidence status at plan commit
No RED or GREEN has run. GitLab and CI source inspection completed; external GitHub job logs and Rust compiler inaccessible here. Phase 02 and whole Telegram migration are not started. No waiver.
