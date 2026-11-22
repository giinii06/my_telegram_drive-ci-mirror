# Phase 00 — Android 제품·빌드 지원 완전 제거

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** APK build/install support no longer exists in source, workflow or current install docs; preserve old release assets and device/user data.

## Repository surface

- `packaging/android/**`
- `.github/workflows/installers.yml`
- `README.md`
- `packaging/README.md`
- `scripts/test_web_only_support.py`
- `.github/workflows/rust-only-cutover.yml`

## Acceptance criteria

- [ ] Delete exactly the 12 tracked Android project files
- [ ] remove android-apk workflow job
- [ ] remove current APK instructions
- [ ] add regression test for no Android packaging/job/advertisement
- [ ] retain Linux/macOS/Windows headless packages.

## Safety / negative cases

- [ ] No deleting historical GitHub Release assets, user data, Telegram sessions, or non-Android packaging. No global Android keyword ban in historical records.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Add Python unittest before implementation; run expected RED, remove Android paths/job/docs, run GREEN. Add the unittest to the existing Rust cutover static contract workflow. Inspect actual exact-SHA job logs; callback-only remains unknown.

## Rollback

Revert only this phase's commit(s). Existing APK release files and local data remain untouched.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
