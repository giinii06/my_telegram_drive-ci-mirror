# Phase 07 — 자동 백업·복구·원본 삭제 안전성

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Backup resumes after crash/network loss without duplicates or data loss; unverified local source is never deleted.

## Repository surface

- `crates/tdrive-backup/**`
- `apps/tdrive-backup/**`
- `crates/tdrive-service/**`
- `tests/e2e/backup/**`

## Acceptance criteria

- [ ] Persistent journal
- [ ] deterministic scan
- [ ] remote completion receipt
- [ ] SHA-256+size verification
- [ ] retry/backoff
- [ ] pause/resume
- [ ] restore
- [ ] retention
- [ ] idempotent schedule.

## Safety / negative cases

- [ ] Size-only equality is insufficient. Unknown upload result, locked vault, expired session, or disk full preserves source.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Synthetic FS/fake API; crash at each journal transition; corrupt hash/partial upload; restore byte equality.

## Rollback

Turn source-delete gate off; retain journal and originals.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
