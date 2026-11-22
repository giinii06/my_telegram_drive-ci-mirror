# Phase 01 — Go 기준선·호환성 oracle·실제 CI 증거

**Kind:** foundation  
**Initial status:** not_started  
**Goal:** All Rust work is compared against deterministic Go behavior without using private production data.

## Repository surface

- `scripts/check-legacy-contracts.py`
- `tests/fixtures/**`
- `.github/workflows/rust-migration-parallel.yml`
- `.github/workflows/rust-only-cutover.yml`
- `docs/migration/compatibility-matrix.md`

## Acceptance criteria

- [ ] Cover 17 TDX1 ops, SQLite v16, TDE1/name seal, HTTP/IPC/CLI, web, backup, media and WebDAV. Fixtures are synthetic or sanitized and language-neutral.

## Safety / negative cases

- [ ] No real Telegram session, production DB, secrets, or remote destructive operation in CI. Dispatch callback is not test proof.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Use existing Go fixture exporters and Rust cargo fmt/clippy/test workflows; inspect exact SHA, provider run, jobs, logs and artifacts. Keep provider classifications separate.

## Rollback

Retain all Go oracle sources until Phase 14; regenerate fixtures from Go tests if needed.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
