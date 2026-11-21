# my_telegram_drive — Rust full-parity implementation plan and evidence

## Approved target and boundaries (continuation, 2026-11-22)

- GitLab authority: `blond11qq/my_telegram_drive` (project 89276294); cumulative review source branch `refactor/rust-phase-07b-tde1-encrypt` @ `4a8bd00a442c2984324a9b88bd5de94edf0fff7c`; this plan's work branch is `refactor/rust-phase-07b-atomic-staging`.
- Requested final state: Rust single backend engine, native GPUI desktop, headless Rust server in Docker; **full** prior Go behavior with compatibility and data-preservation gates.
- Each independently testable subphase gets a plan/checkpoint, implementation commit, matching-SHA **real provider** evidence, two-lens review, then Draft MR. No merge/deployment, force push, live account, real secrets, deletion or unsupported-platform claims.
- Existing master ledger: [docs/migration/phase-ledger.md](../../migration/phase-ledger.md). Detailed 00–15 specs live under `docs/migration/`. This plan is the resumable execution companion, not a second contradictory checklist.

## Verified status at plan creation

Go v16 offline fixture/SQLite reader, limited TDX1 formatter and parser, transport trait and crypto decrypt are **increments**, not end-to-end backend. Recent Draft MRs !16–!19 contain partial Rust config, lock, SQLite backup and streaming TDE1 encrypt/decrypt. Their matching-SHA `github-actions/ci-hub-request` external statuses were `success` for 04.e `7f917801` pipeline 3098178171, 04.f `1970e56b` pipeline 3098178341, 04.g `36258c24` pipeline 3098178424, 07.b `4a8bd00a` pipeline 3098178489. **GitHub job traces are not accessible through the current GitLab-only connector, so do not claim particular GitHub commands ran solely on those statuses.**

The GitLab project uses `ci_config_path=.gitlab/ci-hub-request.yml@blond11qq/ci-hub-smoke`; the only GitLab pipeline job is a manual/no-op `github_actions_request`. A separate GitLab commit status links to a GitHub Actions mirror run. A `skipped` GitLab pipeline does not negate a separately succeeded GitHub status but is not a GitLab Runner test success either. We will never change runner, mirror, CI project visibility or project settings without explicit new authorization.

## Dependency-ordered workstream

1. **S1: Phase 07.b follow-up — atomic private TDE1 file staging** (implemented; external provider success at code SHA, macOS integration pending): [07b2-atomic-file-boundary.md](07b2-atomic-file-boundary.md). Encrypt/decrypt onto 0600 temp file, authenticate then no-clobber publish, durability sync, test wrong key / late corrupt tag / truncation / existing target / symlink / source preservation / large files. No Telegram or vault access.
2. **S2: Phase 07.d1 — Go-compatible sealed filename envelope** (implemented, exact-SHA GitHub provider success; app integration and key lifecycle incomplete): [07d1-name-envelope.md](07d1-name-envelope.md), Go golden vectors plus malformed/wrong key/Unicode/strict base64 tests.
3. **S3: Phase 07.c — vault password and key lifecycle**: Go Argon2id/wrapping fixtures, key rotation, cleanup and redaction; approve only after Go byte-oracle and threat model, then own plan/commit/gates.
4. **S4: Phase 04.f/h — cross-Go/Rust writer exclusion and transactional SQLite writer**: process namespace/OS locking, migrations and crash-safe backup/rollback verified against synthetic Go db. Old Go writer remains authoritative until parity gate.
5. **S5: Phase 05 — real user-account MTProto**: pinned native Rust stack, auth/session/DC, discovery, history and actual binary transport; quarantined test account and deterministic fake. **Do not call transport trait a client.**
6. **S6: Phase 06/08 — 17-op projection writer, replay/sync, uploads/downloads/backup**: normalized Go parity and fault/drain/cancel tests; no remote destructive operation until safety gate.
7. **S7: Phase 09–11 — media, range, WebDAV, real Rust headless HTTP/IPC/CLI/web**, including authenticated E2E.
8. **S8: Phase 12–15 — native GPUI, Docker, OS packaging, benchmark/security, rollback, cutover**; require platform e2e and explicit deployment approval.

**Do not advance a blocked S1 checkpoint as completed.** Other work may be planned and researched read-only, but missing CI proof or a security finding is not waived by making a later commit.

## Phase reporting contract

Each checkpoint must contain: test intent (RED), actual candidate SHA, implemented paths, observed tool/job/test command, passed/failed/skipped/unavailable status, actual external provider run link if available, reviewer findings, and explicit remaining holes. Archive evidence in the phase file and update the ledger with facts, never just ticking a broad Phase from code existence.

## S1 evidence checkpoint (2026-11-22)

- RED acceptance contract committed `f04fdd3ac06b753cbffd16354262945a4b7ee855`. This commit adds tests for not-yet-existent exported functions; no independent red CI run evidence is claimed.
- GREEN candidate `d82edc167482695c10dea351e6deeefe1e5afcb3`, pipeline 3098264188 → `github-actions/ci-hub-request: success`, [GitHub run 44952398680](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44952398680).
- Two-lens self-review found macOS `/var`/`tmp` system symlink compatibility issue; fix `79b520bd26b3f604b332f697e8d2425945ff1ed3`, exact-SHA pipeline [3098266896](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098266896), status `github-actions/ci-hub-request: success`, [GitHub run 44952585082](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44952585082).
- GitLab-only connector cannot independently inspect GitHub job steps; per-job coverage and real Mac runtime remain UNKNOWN. Thus S1 is a **provider-green implementation increment**, not full cross-platform Phase 07.b or product completion. The source Go file and production application have not been modified or wired.

## S2 evidence checkpoint (2026-11-22)

- RED Go/Rust compatibility fixture contract `9381d3a01abd6897d12f2cd88b56fac09a5be5d8` (no independent RED CI run claimed).
- GREEN candidate source `44129cfaf6c94a77a01fa161a1add20e47f2f034` adds `crates/tdrive-crypto/src/name.rs`; synthetic Go test vectors include ASCII/Korean/emoji/max-byte strings.
- Exact source SHA [GitLab dispatch 3098271844](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098271844), separate GitLab commit status `github-actions/ci-hub-request: success` links to [GitHub run 44952988156](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44952988156). Per-job logs are inaccessible through GitLab connector; do not call individual commands independently observed.
- Requirements self-review: Go HKDF salt+info, XChaCha nonce/AAD, strict Base64 URL raw, UTF8 length bounds and key validation align with Go. Security self-review: OS CSPRNG public API; test-only fixed nonce, no secrets in errors; encrypted size discloses length as in Go v1. Missing vault key lifecycle, metadata/key generation replay and real Telegram end-to-end remain P0 before feature release.
