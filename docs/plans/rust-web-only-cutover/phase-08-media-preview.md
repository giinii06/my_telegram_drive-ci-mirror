# Phase 08 — 미디어 Range·썸네일·HLS·remux

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Browser opens image/video/audio previews with range streaming, seek, thumbnails and HLS where supported.

## Repository surface

- `crates/tdrive-media/**`
- `crates/tdrive-files/**`
- `apps/tdrive-server/**`
- `cmd/tdrive/webui/**`
- `tests/e2e/media/**`

## Acceptance criteria

- [ ] Range 206/416
- [ ] bounded reads
- [ ] thumbnails/cache
- [ ] media token expiry
- [ ] HLS/fMP4 remux
- [ ] seek/cancel
- [ ] unsupported codec fallback.

## Safety / negative cases

- [ ] No unbounded whole-file memory load, unauthenticated media token reuse, or decrypted cache access after vault lock.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Go range fixtures; boundary tests; FFmpeg/HLS integration; browser seek/cancel E2E.

## Rollback

Disable media endpoint/flag; preserve original blobs and cache data.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
