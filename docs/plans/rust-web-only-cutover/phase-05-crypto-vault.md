# Phase 05 — TDE1·vault·파일명 암호화

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Browser can create/lock/unlock vault and read/write legacy encrypted files and sealed names without byte drift.

## Repository surface

- `crates/tdrive-crypto/**`
- `crates/tdrive-files/**`
- `crates/tdrive-service/**`
- `apps/tdrive-server/**`
- `tests/fixtures/tde1/**`

## Acceptance criteria

- [ ] TDE1 header/chunks/tags
- [ ] Argon2id
- [ ] HKDF-SHA256
- [ ] XChaCha20-Poly1305
- [ ] sealed filename v1
- [ ] streaming/random access
- [ ] key lifecycle
- [ ] atomic private staging/no-clobber.

## Safety / negative cases

- [ ] Locked vault never falls back to plaintext. Wrong key/tampered tag/truncated chunk fail closed. No key/plaintext in logs/cache.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Go byte fixtures; 0B/1B/64KiB boundaries; large stream; tamper/wrong-key/lock/restart/permissions tests.

## Rollback

Disable crypto writes and preserve all encrypted originals.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
