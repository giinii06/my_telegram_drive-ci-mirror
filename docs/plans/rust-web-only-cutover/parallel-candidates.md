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

## Wave 4 independent candidate scopes

- Phase 01: CI evidence verifier requires the actual Go data/auth/lock step and rejects a response for another requested run ID. Synthetic tests only; no provider switch.
- Phase 11: separate experimental offline Rust inspector Dockerfile, nonroot, locked build, explicit version default. No change to live Compose or Go image; future server packaging consumer.
- Phase 12: bound the total loopback HTTP header read duration, and reject multiply linked token files. No public bind or new routes; future server hardening consumer.
- Phase 13: offline CLI compares all existing core-table digests on two detached v16 snapshots, reports only changed table names and exit status. No production data, no writes or actual canary claim.

Each candidate gets an isolated branch and own CI. Phase 08 provider failure remains unresolved because the external job trace is not available; do not infer a source error from callback alone. No phase advancement, integration, deployment or Go deletion.

### Wave 4 checkpoint

Independent candidate source commits and test-only commits, not integrated into the Draft MR. Requirements and quality/security self-review of source paths only: no production network/data, provider settings, live Compose, or public bind modified. Actual RED/GREEN, full Cargo, Docker build, and browser/E2E execution unavailable here. Phase 12 and 13 both modify main.rs in separate branches: reconcile explicitly at integration. Phase 08 downstream GitLab bridge pipeline #3106077998 is skipped/manual; this is not the failed GitHub job log. Root cause remains unknown.

| Phase | SHA | Pipeline | Provider result | Scope |
|---|---|---|---|---|
| 01 | 6652c8ad5564c26d3ca27004a524607e6081ecd3 | 3106102426 | unknown | CI verifier checks requested run ID and mandatory Go auth/data/lock test step, with synthetic tests invoked by workflow. Actual external job proof remains unavailable. |
| 11 | 4c5e971d4d9b3d3118c42f34fbcf2180d67b5fa4 | 3106102473 | unknown | Experimental separate nonroot locked Rust inspector Dockerfile; no production server/Compose change. Build and standalone Dockerfile contract test not observed. |
| 12 | c2f88167d02113b366b4315c1317facdc65bfa17 | 3106102519 | unknown | Total header deadline for offline loopback HTTP, reject hardlinked bearer token; deadline helper regression test. Socket E2E and release hardening outstanding. |
| 13 | 5092d29695d49e985530d261b1594c490d70124f | 3106102564 | pending | Read-only offline compare CLI for detached Go v16 core table digests; exit failure on invalid snapshot or changed content, fixture-based tests. Full shadow/canary/recovery not performed. |

Previous candidates 02,03,06 now returned provider success; 04 pending at inspection, 08 failure unchanged. None is phase-complete. Phase 14 remains not_started and Go remains intact.

## Wave 5: connect existing Rust components

- Phase 04 continuation, isolated existing wire-payload candidate: apply ordinary multipart part metadata and replay journal together in one SQLite transaction on an explicitly caller-owned disposable clone. First-write wins; reject out-of-order unseen operations and unsupported renditions; do not use as full sync, do not touch a live database. Test duplicate, tamper, rollback, and non-part rejection. Consumer: future history synchronizer.
- Phase 06 continuation, isolated existing multipart-read candidate: expose complete multipart metadata via actual tdrive-inspect CLI --list-parts, lossless decimal-string IDs, no partial output for incomplete data, no DB writes. Consumer: later download service. Test actual binary with detached synthetic snapshot.

These scopes reuse existing approved phases and branch histories. No integration into Draft MR, Go removal, remote account access or deployment. Existing provider callbacks do not establish full test evidence. Workstreams have separate branches; no independent agent claim.

### Wave 5 checkpoint

Both workstreams continue existing isolated branches, not new disconnected crates. Requirements self-review: part write and replay are in the same transaction; CLI directly consumes prior storage lookup. Security/quality self-review: no remote or live DB writes, rejects unsupported part attributes, no partial JSON output on invalid snapshot. Risks: canonical payload currently a supported subset, not full Go JSON digest equivalence; all 17 operations, full integration, real Rust tests and exact provider job proof still outstanding. Phase 04 and 06 modify storage lib exports in their separate branches and require reconciliation at integration. RED/GREEN not executed locally; no rustc/cargo in current container. No independent reviewers. No waiver, merge or deployment.

| Phase | Implementation SHA | Pipeline | Provider |
|---|---|---|---|
| 04 | 66ca7485da68c0db466513ca95656f2b88b1a457 | 3106109598 | unknown |
| 06 | 041b6b2e7a38b460ddf0ead29e573ea5bbcc91cb | 3106109754 | unknown |

Previous Wave 4 (01,11,12,13) exact candidate provider callbacks now success; no per-job evidence. Phase 08 failed callback unchanged; actual failing GitHub job log unavailable, so no speculative CI repair. Phase 14 remains gated. Go production untouched.
