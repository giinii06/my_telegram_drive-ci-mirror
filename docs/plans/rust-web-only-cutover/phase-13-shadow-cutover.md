# Phase 13 — 비파괴 Shadow·격리 Canary·복구 리허설

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Without touching Go production, prove snapshot parity, isolated Telegram E2E, data integrity and recovery.

## Repository surface

- `tests/e2e/**`
- `scripts/migration/**`
- `docs/operations/migration/**`
- `apps/tdrive-server/**`

## Acceptance criteria

- [ ] Table/tree/crypto hashes
- [ ] upload/download
- [ ] backup
- [ ] media
- [ ] WebDAV
- [ ] browser flows
- [ ] crash/fault recovery
- [ ] restore
- [ ] rollback on each supported target.

## Safety / negative cases

- [ ] Never run Go/Rust on same data dir or Telegram account. No production volume/account/cutover/deploy without separate approval.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Detached read-only snapshots; disposable account/channel; exact-SHA E2E; fault injection/restore drill; independent review.

## Rollback

Leave Go service and volume unchanged; preserve only privacy-safe comparison evidence.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
