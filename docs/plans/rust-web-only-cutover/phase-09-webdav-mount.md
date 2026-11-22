# Phase 09 — WebDAV·마운트·외부 파일 프로토콜

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Existing WebDAV protocol clients retain the same authenticated file contract through Rust.

## Repository surface

- `crates/tdrive-mount/**`
- `crates/tdrive-api/**`
- `apps/tdrive-server/**`
- `backend/mountdav/** (oracle only)`
- `tests/compatibility/webdav/**`

## Acceptance criteria

- [ ] PROPFIND/GET/HEAD/PUT/MOVE/COPY/DELETE
- [ ] auth/path/range/ETag/lock
- [ ] read/write
- [ ] vault policy
- [ ] concurrency/reconnect. OS-specific mount adapters are decided from actual feature inventory.

## Safety / negative cases

- [ ] No path traversal, cross-user access, plaintext writes while vault locked, or unsafe overwrite/delete. Do not remove WebDAV because the client UI is web-only.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Go endpoint fixtures; WebDAV protocol tests; auth/path matrix; concurrent writer/disconnect tests.

## Rollback

Switch WebDAV writes to read-only and preserve journal/remote operations.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
