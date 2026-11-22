# Phase 03 — Telegram 사용자 계정 MTProto·세션·전송

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Browser onboarding establishes a Rust-owned Telegram user session with Go-equivalent access and transfer behavior.

## Repository surface

- `apps/tdrive-mtproto/**`
- `crates/tdrive-telegram/**`
- `crates/tdrive-config/**`
- `apps/tdrive-server/**`
- `tests/integration/telegram/**`

## Acceptance criteria

- [ ] Phone/code/2FA/logout/session recovery
- [ ] channel access_hash
- [ ] history/difference
- [ ] upload/download/multipart
- [ ] FLOOD_WAIT
- [ ] timeout/cancel/reconnect
- [ ] stable random_id.

## Safety / negative cases

- [ ] No Bot API replacement. Never expose session/credentials in argv/log/artifacts. Ambiguous send outcome must be reconciled, not blindly resent.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Fake MTProto tests; nested lockfile; cargo fmt/clippy/test/build; disposable isolated account E2E and byte digest equality.

## Rollback

Stop remote writes; recover only with isolated account/channel; never run two writers for one account.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
