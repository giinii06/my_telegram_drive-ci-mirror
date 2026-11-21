# S1 / Phase 07.b2 — atomic TDE1 on-disk file I/O

**State at plan commit:** PLANNED; no implementation or post-plan CI evidence yet.

## Why this is a distinct phase

The existing `tdrive-crypto::encrypt_stream` and `decrypt_stream` accept arbitrary `Write`. On an authentication error, an external caller can have already received plaintext from previously authenticated chunks. The crypto parser is not a safe downloadable file writer until complete authentication and a no-clobber publication boundary are built. The Go source of truth is `backend/crypto/stream.go` and `backend/crypto/random_access.go`; the Rust source is `crates/tdrive-crypto/src/{stream,auth}.rs`.

## Code shape and security contract

- New `crates/tdrive-crypto/src/file_io.rs` exported as `encrypt_file_atomic` and `decrypt_file_atomic`, Unix-gated initially. Only explicit absolute paths accepted; refuse final source symlinks, output parent symlinks/insecure directories, and any already-existing destination.
- Source opened read-only; immutable source length used for encryption size. The target directory must preexist, be private 0700, and reside on the same volume as the temp file. Create only a randomized 0600 `create_new` staging file inside target parent; never write directly into a visible final target.
- On success: `flush`/sync output, authenticate final TDE1 marker for decrypt, optionally verify header/size, atomically no-clobber `hard_link(temp,destination)`, unlink temp, sync parent. Temporary material cleaned up with RAII on any error. If final target appears during execution, fail without replacing it.
- Fail closed on wrong key, late-tag tamper, missing final marker, invalid TDE1 header/size, truncated source, existing destination, symlink destination, 0755 output parent and I/O exceptions; preserve input and operator-owned target bytes.
- No secret value logs; generic key length guards. No real vault key, actual Telegram session, remote upload or user database used in tests.

## RED / GREEN / REVIEW / CI

- RED: tests create a synthetic Go-compatible TDE1 file, corrupt a *late* chunk (after earlier authenticated bytes), assert no published output and no temp leaks. Also test 9 MiB+ roundtrip, no-clobber and preservation.
- GREEN: independently implemented `file_io.rs` with small constant-size buffers; tests validate SHA/content equivalence and source preservation.
- Review lens A: meets requested behavior and Go wire parity, no implicit destructive operations.
- Review lens B: permissions/TOCTOU, resource bounds, zeroization, durability and cross-platform fallbacks. Residual races need explicit documentation.
- Verification: candidate commit -> exact-SHA pipeline request -> look up `github-actions/ci-hub-request` status with matching pipeline ID. GitLab Runner output if any, GitHub per-job logs only if actually accessible. Fix only proven failures and recount repair attempts.
- Checkpoint commit only after real evidence; completion label requires all tests and review gates. Windows and production integration remain outside this subphase.

## Evidence ledger

| Event | Commit SHA | Evidence | Status |
|---|---|---|---|
| Plan committed | recorded via GitLab commit | GitLab review branch | PLANNED |
| RED test | pending | none | NOT STARTED |
| Candidate implementation | pending | none | NOT STARTED |
| Provider verification | pending | none | NOT VERIFIED |
| Dual-lens review | pending | none | NOT REVIEWED |
