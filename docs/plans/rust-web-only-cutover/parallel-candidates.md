# Parallel implementation candidates

User explicitly requested parallel implementation of all phases. Existing Web-only plan and acceptance criteria remain authoritative. Parallel candidate development is not a waiver of completion, CI or E2E gates. Use independent branches from a common reviewed base, without merging or deploying. No production account/data access.

First independent workstreams: Phase 07 backup digest verification (tdrive-backup only); Phase 08 bounded range streaming (tdrive-media only); Phase 09 WebDAV lock-header contracts (new tdrive-mount crate, root Cargo manifests). Test candidates first, implementation commits second, exact-SHA CI separately. Other phases remain in existing ledger until their code candidates exist. No agent independence is claimed; actual tool calls may run concurrently while source review is performed by one assistant. Final integration and Go deletion remain dependency-gated.

## Candidate checkpoint

All candidates are isolated from the Draft MR branch; no integration/merge/deploy. Test-only commits exist, but RED and GREEN were not executed locally (rustc/cargo absent) and individual CI job logs unavailable. No phase is marked implemented in full/tested/verified/completed. The user's parallel execution direction authorizes candidate work, not a waiver of verification. Sequential requirements and quality/security self-review of diffs only, no independent reviewer.

| Phase | Candidate | Pipeline | Provider | Scope |
|---|---|---|---|---|
| 05 | [a08a20c1](https://gitlab.com/blond11qq/my_telegram_drive/-/commit/a08a20c112464adeb47126f4be451d1573309674) | [3106080986](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3106080986) | unknown | Go-compatible XChaCha20-Poly1305 master-key wrap/unwrap and key-check envelopes; Argon2id, JSON metadata, key ownership and Go golden vectors still outstanding. |
| 07 | [ec075b06](https://gitlab.com/blond11qq/my_telegram_drive/-/commit/ec075b06cdbfa97584f0319a9c19b60599060d22) | [3106077901](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3106077901) | pending | Require matching SHA-256 in deletion guard and re-observed original metadata; trusted digest measurement, journal persistence and real backup/restore still outstanding. |
| 08 | [962f47e1](https://gitlab.com/blond11qq/my_telegram_drive/-/commit/962f47e126dd871f5d6611e2d6f6d7e271198353) | [3106077952](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3106077952) | pending | Bounded range copy from seekable source, invalid-range no read, short source rejected; actual media endpoint/auth/cache/cancellation/HLS still outstanding. |
| 09 | [1bce8732](https://gitlab.com/blond11qq/my_telegram_drive/-/commit/1bce87324055616c566e0079f42dd9d997c7be28) | [3106078024](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3106078024) | pending | Rust mount crate with Go-compatible LOCK Depth and Timeout parse rules; lock state, DAV handlers, auth, path normalization and integration still outstanding. |
| 10 | [fa493346](https://gitlab.com/blond11qq/my_telegram_drive/-/commit/fa493346fac7a22664eadfd470982c94ab61a9d6) | [3106081056](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3106081056) | unknown | Harden existing read-only HTTP parser for missing/duplicate/empty Host, control bytes in header values and invalid configured bearer token; full API parity/browser integration still outstanding. |

## Wave 2: independent implementation scopes

- Phase 05: branch `refactor/rust-only-parallel-vault-envelope` corrected to `refactor/rust-only-parallel-05-vault-envelope`; provider result success, jobs/E2E unknown.
- Phase 07: branch `refactor/rust-only-parallel-backup-digest` corrected to `refactor/rust-only-parallel-07-backup-digest`; provider result success, jobs/E2E unknown.
- Phase 08: branch `refactor/rust-only-parallel-bounded-media` corrected to `refactor/rust-only-parallel-08-bounded-media`; provider result failed, jobs/E2E unknown.
- Phase 09: branch `refactor/rust-only-parallel-webdav-lock` corrected to `refactor/rust-only-parallel-09-webdav-lock`; provider result success, jobs/E2E unknown.
- Phase 10: branch `refactor/rust-only-parallel-http-boundary` corrected to `refactor/rust-only-parallel-10-http-boundary`; provider result success, jobs/E2E unknown.

- Phase 02 (tdrive-config): reject multiply-linked private files; validate the opened descriptor against pre-open metadata on Unix. Later consumer: native service/session config; no production credentials.
- Phase 03 (tdrive-telegram): send control once using persisted random ID, reconcile ambiguous outcomes through an explicit read port, fail closed on unresolved delivery; later consumer: MTProto write adapter. No real Telegram account writes.
- Phase 06 (tdrive-storage): resolve complete ordered multipart data from detached snapshot and validate manifest count/size/index; later consumer: streaming download. No DB writes.

Keep candidate branches isolated and phase completion gates unchanged. Actual CI failure for Phase 08 remains under investigation; do not infer failure cause from provider callback alone.

### Wave 2 checkpoint

Three isolated candidates, source review only; Rust compiler/Cargo unavailable locally. Test-only commits exist; RED/GREEN execution not observed. Provider callbacks are not job evidence. No merge or deploy.

| Phase | Code SHA | Pipeline | Provider | Scope |
|---|---|---|---|---|
| 02 | 22ccc86ebad673d6d4f9b10eda3247f90b33cbc7 | 3106090923 | pending | Reject hardlinked session/config files and compare opened descriptor identity. Windows ACL, data-store runtime integration remain. |
| 03 | 84f8d4c846b7b3d4f13ec8e4de501b982d8624de | 3106090987 | pending | Send once using persisted random ID; explicit reconciliation port returns receipt or ambiguous outcome without automatic resend. Real MTProto adapter/journal still outstanding. |
| 06 | cc4cb1683ac7cc9b6ff714a0a48b43655b834edc | 3106091059 | pending | Detached read-only ordered multipart lookup, channel scope, count/size/index/overflow checks. Transfer service, revisions and real remote reads still outstanding. |

### Wave 3 scope

Phase 04: retain currently discarded Go TDX1 write payload fields in the Rust parser (multipart IDs/index, CAS revisions, overwrite destination and retention metadata, protocol/key versions and plaintext size), with regression tests and fail-closed simple formatter. Consumer: eventual transactional projector; do not enable live replay or destructive writes. Isolated from prior branch work.

### Wave 3 checkpoint and live provider status

Phase 04 source diff reviewed against Go wire.go and existing formatter. Test-only and implementation commits are separate. No actual Rust execution here (rustc/cargo unavailable); no verified full checkout or E2E. Requirements review: preserve parsed fields, retain existing validation and reject unsupported formatter payloads. Quality/security review: no new database or network writes; encrypted payload application and compatibility still require further tests. Self-review only.

| Phase | Candidate SHA | GitLab pipeline | Provider result |
|---|---|---|---|
| 02 | 22ccc86ebad673d6d4f9b10eda3247f90b33cbc7 | 3106090923 | pending |
| 03 | 84f8d4c846b7b3d4f13ec8e4de501b982d8624de | 3106090987 | pending |
| 04 | dff9179e6a893248a97d7efd115cfc18124fdcb3 | 3106095246 | unknown |
| 05 | a08a20c112464adeb47126f4be451d1573309674 | 3106080986 | success |
| 06 | cc4cb1683ac7cc9b6ff714a0a48b43655b834edc | 3106091059 | pending |
| 07 | ec075b06cdbfa97584f0319a9c19b60599060d22 | 3106077901 | success |
| 08 | 962f47e126dd871f5d6611e2d6f6d7e271198353 | 3106077952 | failed |
| 09 | 1bce87324055616c566e0079f42dd9d997c7be28 | 3106078024 | success |
| 10 | fa493346fac7a22664eadfd470982c94ab61a9d6 | 3106081056 | success |

Provider callback is not actual job evidence. Failed candidates remain blocked for failure-log inspection, not fixed speculatively. No candidate is merged, verified or completed.
