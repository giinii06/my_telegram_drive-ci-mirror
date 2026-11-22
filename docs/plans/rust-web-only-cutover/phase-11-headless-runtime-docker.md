# Phase 11 — Headless Rust server·Docker Compose·Tailscale

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Linux Docker server serves the browser UI over Tailscale and preserves data through restart/upgrade/restore.

## Repository surface

- `Dockerfile`
- `docker/entrypoint.sh`
- `docker-compose.yml`
- `install.sh`
- `apps/tdrive-server/**`
- `docs/operations/**`

## Acceptance criteria

- [ ] Linux amd64/arm64
- [ ] non-root
- [ ] /data ownership
- [ ] health/readiness
- [ ] graceful shutdown
- [ ] persistent SQLite/WAL/session/vault
- [ ] Tailscale sidecar
- [ ] backup/restore/upgrade smoke.

## Safety / negative cases

- [ ] Never run compose down -v. No auth key/secret in image/log. No public exposure of tailnet-only no-auth service.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Cargo release build; docker buildx; compose config; no-secret help/status smoke; disposable account E2E; volume restore.

## Rollback

Revert to known-good Go image/compose after stopping Rust; never share a live volume.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
