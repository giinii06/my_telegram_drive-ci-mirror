# Rust offline TDE1 CLI — first executable vertical slice

- Parent review branch: `ci/rust-parallel-workflow-repair-20261122` at `77ba022094b863342d49da37ff44d691f9be9dd3`.
- Work branch: `feat/rust-offline-tde1-cli-20261122`. Keep Draft MR; **never merge/deploy**.
- User outcome: preserve existing Go encrypted file format while exposing a real Rust entrypoint that can encrypt/decrypt files offline, with wrong-key/tamper and overwrite failure protection. This is **not** a Telegram uploader or finished Rust migration.
- Approved migration design: `docs/plans/my-telegram-drive-rust-migration/README.md` S1/S2 and `docs/migration/phase-ledger.md`; no new backend architecture, vault system or CI provider changes.
- Gate version: 2.2; existing CI bridge's `dispatch`/external callback is insufficient to attest Rust tests.

## Phases
| Step | Kind | Outcome | Status |
| --- | --- | --- | --- |
| 01 | user_flow (offline only) | `tdrive-inspect --encrypt-file SRC OUT KEYFILE` and `--decrypt-file` use the existing atomic TDE1 Rust crate and preserve files on error, with a 32-byte 0600 key file, no key on command line | implemented (not tested/verified) |
| 02 | integration | Rust native CLI actually compiles/tests and performs successful + negative end-to-end offline roundtrip; exact SHA real job evidence | not_started |
| 03 | user_flow | Real isolated Telegram account upload/download, vault, GPUI, Docker, legacy parity/rollback per master migration plan | not_started and not delivered |

## Safety and explicit limits
This raw-key CLI is a nonproduction migration diagnostic only. It requires a preexisting private absolute directory (0700) for its output, refuses existing targets and unsafe sources via `tdrive-crypto::file_io`, and checks the key file is a regular non-symlink Unix file with owner-only access (0600 or more restrictive). It never creates/manages the vault, provides no key derivation, does not transmit to Telegram, and never prints the raw key. It never changes the Go application, user data, permissions, mirror, runners, credentials or deployment.

No Rust compiler, private GitLab repository checkout or GitHub six-job trace was available at plan time. Missing RED/GREEN executable test evidence is reported as unverified; no phase is completed until real compiled binary and user-flow tests run. Do not mistake file/line counts or CI dispatcher status for full production readiness.

Detailed implementation acceptance: [phase-01-offline-cli.md](phase-01-offline-cli.md).

## Current candidate and proof level

- Actual Rust CLI implementation candidate: `970dafcc04a7f1e008bb5117ac5aff72b4a9ec7e` on Draft MR !23 (this branch targets CI MR !22).
- Added tests specify four real CLI behaviors, but **have not executed** without Rust toolchain. Observed GitLab provider is dispatch-only pipeline #3098837254; do not assume Cargo ran.
- No other migration stage advanced. See [Phase 01 checkpoint](phase-01-offline-cli.md) and the 2.2 [state ledger](phase-state.json).
