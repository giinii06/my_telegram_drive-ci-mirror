# Phase 14 — 완전한 parity 후 Go source·toolchain 퇴역

**Kind:** foundation  
**Initial status:** not_started  
**Goal:** Rust-only checkout no longer needs Go for product runtime, build, test, package or operations.

## Repository surface

- `backend/**/*.go`
- `cmd/**/*.go`
- `go.mod`
- `go.sum`
- `Dockerfile`
- `.github/workflows/**`
- `packaging/**`
- `scripts/**`
- `tests/fixtures/**`
- `README.md`

## Acceptance criteria

- [ ] Every Go-owned flow has verified Rust replacement
- [ ] tracked Go source=0
- [ ] go.mod/go.sum absent
- [ ] Go CI/dependency/package/runtime steps=0
- [ ] language-neutral golden fixtures retained.

## Safety / negative cases

- [ ] Do not delete Go while any Phase 00–13 gate is incomplete/unknown. Keep historical fixtures and wire-format documentation where needed.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Clean checkout cargo fmt/clippy/test/build; Bun lint/unit/browser; Docker/Compose smoke; packages; Go-source/runtime audit; restore drill.

## Rollback

Dedicated final removal commits and Draft MR only. No production migration, merge or deployment.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
