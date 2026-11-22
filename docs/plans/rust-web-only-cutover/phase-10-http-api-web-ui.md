# Phase 10 — Rust HTTP API와 브라우저 웹 UI 완전 연결

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Desktop and mobile browsers complete onboarding, browse/search, upload/download, media, settings and history flows using Rust only.

## Repository surface

- `crates/tdrive-api/**`
- `apps/tdrive-server/**`
- `apps/tdrive-cli/**`
- `cmd/tdrive/webui/**`
- `tests/e2e/web/**`

## Acceptance criteria

- [ ] Exact routes/method/query/body/cookie/error/status/stream parity
- [ ] onboarding/auth/CSRF/Origin
- [ ] upload progress/events
- [ ] pagination/file URLs
- [ ] browser history
- [ ] responsive layout/accessibility. Svelte/Bun retained
- [ ] no GPUI.

## Safety / negative cases

- [ ] No token leakage, CSRF, open redirect, unsafe CORS, path traversal, unbounded body or stale media token.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Bun frozen install/lint/unit/Playwright desktop+mobile; Rust HTTP integration; Go route snapshots; 401/403/404/409/413/416/429/500 matrix.

## Rollback

Use compatibility adapters/flags; keep one Rust data writer.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
