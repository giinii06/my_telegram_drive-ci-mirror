# S2 / Phase 07.d1 — hidden-name v1 Go-compatible crypto

**At plan commit:** PLANNED. Separate independently testable Rust library implementation, no key lifecycle or live remote integration.

## Go oracle and byte layout

Authoritative `backend/crypto/name.go` and `backend/crypto/name_test.go`:
- Master key length exactly 32 bytes, filename is valid UTF-8, nonempty, at most 4096 **bytes** (not Unicode code points).
- HKDF-SHA256 salt = ASCII `tdrive.name-seal.v1`, info = ASCII `tdrive/file/v1`, 32-byte subkey.
- AEAD XChaCha20-Poly1305; 24-byte **random per-envelope** nonce; AAD = `v1`; ciphertext = name bytes + 16-byte tag.
- Envelope format `v1:` followed by unpadded URL-safe Base64 of `nonce || ciphertext || tag`. Unknown versions/wrong key/tamper/invalid UTF-8/empty plaintext reject.
- `NameKeyVersion=1`; the Go projection is the source of truth for version handling. Do not implement a different incompatible format.

## Implementation and actual byte oracle

- `crates/tdrive-crypto/src/name.rs`: exported `encrypt_file_name`, `decrypt_file_name`, and `NAME_KEY_VERSION`. OS random nonce on public path; only test-only helper accepts a deterministic nonce.
- No new dependencies: existing `chacha20poly1305`, `hkdf`, `sha2`, `rand_core`. Base64 URL raw codec kept bounded, strict and independently differential-tested against Go; errors redact filenames/key/ciphertext.
- Add Go fixture exporter `TestExportRustNameSealFixtures` to produce six synthetic cases at controlled 24-byte test nonce and fixed sample 32-byte key; Rust CI loads Go-generated fixtures and compares complete envelope bytes, plus test for wrong key, tamper, bad-version, malformed Base64, max length, boundary Unicode and randomized nonrepeatability.
- Fixtures are generated during GitHub Actions and uploaded to `rust-contracts`, not committed secrets. Deterministic nonce is **never** exposed in public Rust encryption API.

## Scope/acceptance gate

- No Argon2id vault password, master key storage, remote Telegram, actual files named on disk or production GPUI integration.
- Individual test: Rust reproduces Go seed vectors, Go can decrypt Rust envelope with known exact nonce (byte equality implies decrypt compatibility), Rust rejects malformed/tampered envelope.
- Exact-SHA CI: GitLab pipeline dispatch ID and corresponding `github-actions/ci-hub-request` status, independent GitHub run check if connector available; no false per-job claims from callback alone.
- Two-lens self-review (or real reviewer if available), status and remaining risks checkpoint, separate Draft MR, no merge or deployment.

## Evidence
| Stage | SHA | Result |
|---|---|---|
| Plan | record via commit | PLANNED |
| RED contract | pending | NOT RUN |
| GREEN code | pending | NOT VERIFIED |
| CI | pending | UNKNOWN |
| Review | pending | NOT DONE |
