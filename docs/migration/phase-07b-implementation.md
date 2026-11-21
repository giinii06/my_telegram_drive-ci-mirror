# Phase 07.b — Rust TDE1 streaming encryption writer (increment)

## Go source contract
backend/crypto/stream.go: TDE1 v1 = 50 byte header, 16-byte random salt, 20-byte random nonce prefix, 64-KiB chunks, XChaCha20-Poly1305, HKDF-SHA256("tdrive/file/v1"), BE32 counter with final bit in bit31, explicit empty 16-byte final tag for empty or exact block multiple. Exact output size: 50 + plaintext + (floor(size / 65536)+1) * 16.

## Rust implementation
crates/tdrive-crypto/src/stream.rs exports encrypt_stream<R:Read,W:Write>. Generates per-file entropy with rand_core OsRng (getrandom enabled; pinned version 0.6.4), derives a per-file 32-byte subkey using existing HKDF crate, and writes fixed-size chunks rather than buffering the entire file. The header and nonce bits exactly mirror Go, including full-block boundaries. The Rust writer validates master key length and declared plaintext length, refuses short/extra streams, and zeroes the temporary HKDF key, chunk buffer and ciphertext memory after use.

## Independent Go↔Rust comparison
backend/crypto/rust_tde1_fixture_test.go generates six deterministic synthetic TDE1 files with a **test-only** 36-byte entropy stream (16 salt bytes 0x11 + 20 prefix bytes 0x22). Rust unit test calls the private fixed-material function on the same plaintext, compares the entire ciphertext byte-for-byte to Go output for sizes 0,1,65535,65536,65537,196625, and also verifies Rust authenticated decrypt round trips. Random Go fixtures remain unchanged and continue validating Go→Rust decryption. GitLab and GitHub CI artifact checks require seeded fixture zero case.

## Safety and incomplete work
- The public writer uses OS entropy; deterministic material is private to its unit tests. NEVER reuse deterministic salt/prefix with a real master key.
- On I/O or declared-length error, the output writer may contain a **partial ciphertext**. Callers must write to a private temp file and publish only after success; there is no Telegram upload / local vault integration.
- This does not yet add authenticated streaming decryption with private temp-file staging, vault key lifecycle, password KDF, key rotation, fully verified media range, or production E2E. Complete Phase 07.b only after these stage/restore/error/fault checks. All CI gates must match the exact HEAD SHA.

## Streaming decrypt companion

The same partial Phase 07.b implementation now includes decrypt_stream: it authenticates each 64-KiB chunk, requires the explicit authenticated final marker and validates plaintext length. Tests cover chunk boundaries, wrong keys, truncated final tags and 9-MiB streams. This is not a production file publishing API: an error in a later chunk may leave earlier authenticated bytes in the caller output; callers must privately stage and publish only after complete success.
