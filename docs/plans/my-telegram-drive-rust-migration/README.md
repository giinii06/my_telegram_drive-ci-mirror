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

1. **S1: Phase 07.b follow-up — atomic private TDE1 file staging** (active): [07b2-atomic-file-boundary.md](07b2-atomic-file-boundary.md). Encrypt/decrypt onto 0600 temp file, authenticate then no-clobber publish, durability sync, test wrong key / late corrupt tag / truncation / existing target / symlink / source preservation / large files. No Telegram or vault access.
2. **S2: Phase 07.c — vault password and key lifecycle**: Go Argon2id/wrapping fixtures, key rotation, cleanup and redaction; approve only after Go byte-oracle and threat model, then own plan/commit/gates.
3. **S3: Phase 04.f/h — cross-Go/Rust writer exclusion and transactional SQLite writer**: process namespace/OS locking, migrations and crash-safe backup/rollback verified against synthetic Go db. Old Go writer remains authoritative until parity gate.
4. **S4: Phase 05 — real user-account MTProto**: pinned native Rust stack, auth/session/DC, discovery, history and actual binary transport; quarantined test account and deterministic fake. **Do not call transport trait a client.**
5. **S5: Phase 06/08 — 17-op projection writer, replay/sync, uploads/downloads/backup**: normalized Go parity and fault/drain/cancel tests; no remote destructive operation until safety gate.
6. **S6: Phase 09–11 — media, range, WebDAV, real Rust headless HTTP/IPC/CLI/web**, including authenticated E2E.
7. **S7: Phase 12–15 — native GPUI, Docker, OS packaging, benchmark/security, rollback, cutover**; require platform e2e and explicit deployment approval.

**Do not advance a blocked S1 checkpoint as completed.** Other work may be planned and researched read-only, but missing CI proof or a security finding is not waived by making a later commit.

## Phase reporting contract

Each checkpoint must contain: test intent (RED), actual candidate SHA, implemented paths, observed tool/job/test command, passed/failed/skipped/unavailable status, actual external provider run link if available, reviewer findings, and explicit remaining holes. Archive evidence in the phase file and update the ledger with facts, never just ticking a broad Phase from code existence.
