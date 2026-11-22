# Phase 01 — offline CLI wiring into atomic TDE1

**Kind:** user_flow (offline diagnostic), **Status:** implemented (unverified), **gate_version:** 2.2.

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

## Source-exact implementation checkpoint (2026-11-22)

- **Plan:** `b3022ede4ffc2ca2d62623e0df05107eaa6f0445` on a separate stacked branch; RED integration tests `4d852d33331164f92bb09c08c160f3f317b14868`, deterministic last-tag tamper repair `8ecf7c5f60221e634daa137388aeb1bfb0c8a47f`. No Rust RED process executed.
- **Code:** `38f0e8f8a8f19fb1b2e1a4fe906c63095e35484d` adds `apps/tdrive-inspect/src/offline_crypto.rs`, two Unix-only CLI flags, and adds `tdrive-crypto` path dependency. Follow-up `970dafcc04a7f1e008bb5117ac5aff72b4a9ec7e` updates `Cargo.lock` for `--locked` compatibility. This is the **current candidate code SHA**.
- **Static review:** re-read actual GitLab `main.rs`, `offline_crypto.rs`, Cargo manifest and four integration tests. Crypto operations reuse already existing `tdrive-crypto::encrypt_file_atomic`/`decrypt_file_atomic` with its private temporary, final AEAD and no-clobber publication rules; the CLI never displays a 32-byte key or passes it as an argv value. Key path/file checks cover absolute path, no dot components, regular non-symlink 0600 or stricter, stable dev/inode, exact 32 bytes. The module remains a **nonproduction diagnostic**; no vault management, key derivation, OS memory lock or MTProto. Source file path UTF-8 CLI arguments only (existing `std::env::args` behavior), Windows feature unavailable.
- **Source-exact CI:** [GitLab pipeline #3098837254](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098837254) at code SHA `970dafcc04a7f1e008bb5117ac5aff72b4a9ec7e` reported `success`, `dispatch_github_actions=success`. GitLab Runner job count **zero**, GitHub run and job test logs **not observed**. Classification: `dispatch=success`, `provider_result=unknown`, `jobs_verified=unknown`, `e2e_verified=unknown`.
- **Commands NOT executed in this environment:** `cargo +1.92.0 fmt --all --check`; `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`; `cargo +1.92.0 test -p tdrive-inspect --test offline_file_cli --locked`; `cargo +1.92.0 test --workspace --locked`. No Rust compiler or private source checkout; a dispatch status is not execution proof.
- **Review:** [reviews/phase-01.json](reviews/phase-01.json), two separate sequential **self-review** lenses at exact code SHA, no independent reviewer or executed app. The most significant open verification risk is untested compile/whole-app runtime and platform-specific file behavior, not an observed defect.
- **State:** Phase 01 `implemented` but **not tested/verified/completed**; Phase 02 actual compiled Rust user-flow integration `not_started`; Phase 03 Telegram, GPUI, Docker full Go parity `not_started`. No waiver, merge, deployment, user data, CI provider/runner, secrets or mirror changes.

## Developer verification once an authorized Rust executor and private checkout exist

```sh
cargo +1.92.0 fmt --all --check
cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.92.0 test -p tdrive-inspect --test offline_file_cli --locked
cargo +1.92.0 test --workspace --locked
```

This is a *runbook*, not a claim that these commands have been executed. After GREEN, test on target macOS against disposable files, recheck current branch exact SHA, and collect real GitHub job logs if the CI bridge can expose them. No live Telegram account/production data test is authorized.
