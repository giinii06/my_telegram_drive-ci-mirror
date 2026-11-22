# Phase 06 — 파일·폴더·revision·업로드/다운로드

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Browser performs every existing file operation through Rust services, including large resumable transfers.

## Repository surface

- `crates/tdrive-files/**`
- `crates/tdrive-service/**`
- `crates/tdrive-storage/**`
- `crates/tdrive-telegram/**`
- `apps/tdrive-server/**`
- `tests/e2e/files/**`

## Acceptance criteria

- [ ] Browse/create/rename/move/copy/trash/restore/purge
- [ ] multipart
- [ ] progress/resume/conflict/cancel
- [ ] hash/size
- [ ] revisions
- [ ] API/CLI semantics match Go.

## Safety / negative cases

- [ ] No success before remote completion. No delete before full verification. Hard delete requires explicit confirmation and idempotency.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Service tests; multipart fault injection; API integration; large transfer/resume; Go snapshot comparison.

## Rollback

Disable write flags and resume from durable operation journal.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
