# Phase 07 — Go TDE1 cryptographic compatibility (bounded read-only increment)

Phase 07 now includes a Rust XChaCha20-Poly1305 **authenticated decryptor** for
complete TDE1 test streams up to **8 MiB**, alongside the previous 50-byte header
and ciphertext-size inspector. The implementation is grounded in the existing
Go backend/crypto/stream.go layout:

- 32-byte synthetic test master key; HKDF-SHA256 per-file key with 16-byte
  file salt and info "tdrive/file/v1"
- XChaCha20-Poly1305 24-byte nonce: 20-byte header prefix + big-endian
  u32 counter with the top bit marking the final authenticated chunk
- 64 KiB plaintext chunks + 16-byte tags, with an additional authenticated
  empty final chunk when the file length is a multiple of 64 KiB
- verify the total declared length, exact stored size and **every** tag
  before returning any plaintext

Synthetic Go fixtures are created by its production EncryptStream at CI time.
Sizes 0, 1, 65535, 65536, 65537 and 196625 exercise empty, short, full and
multi-chunk boundaries; Rust tests validate content, wrong key, tampering,
truncation and appended bytes. Fixtures contain no production data or keys and
are not committed to Git.

**Still excluded**: production streaming decrypt/encrypt APIs, Argon2id vault
creation/unlock, master key rotation, encrypted filename envelopes, secure
filesystem staging, hardened key lifecycle, versioned authentication context,
real Telegram upload or live SQLite migration. This 8-MiB reader is intended
for compatibility tests ONLY. Its success is not full Phase 07 completion.

No Go encryption implementation or production routing is modified. Do not
replace the existing encrypted-drive service with this testing reader.
