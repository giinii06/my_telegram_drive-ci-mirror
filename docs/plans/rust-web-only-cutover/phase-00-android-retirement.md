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


## Execution record — 2026-11-23

- **Test-only candidate:** `d64de6c4c094bee751a1a39f1896fcbde1bf8c30`; corrected regex in `8d3454e87c97dbeabe65718ef84768696ab2fa43`.
- **Local RED evidence:** the same 4-test contract was run against a reduced temporary source-snapshot fixture seeded with the current Android path/job/APK markers. Result: 3 expected failures and 1 pass. This is not a full repository checkout and is not recorded as full-checkout RED.
- **Implementation:** `361f88de27d02d88b113ee728b59ada1e101584c` deletes 12 tracked `packaging/android/` files, removes the `android-apk` installer job, removes APK instructions from current user/package READMEs, and retains all four non-Android headless installer jobs.
- **Source audit:** current tree contains zero `packaging/android/` files; current root/package README and installer workflow contain zero Android/APK references; remaining installer jobs are `linux-deb`, `macos-dmg`, `windows-exe`, `windows-backup-exe`.
- **CI:** pipeline #3100313264 on the exact implementation SHA reports bridge success only. Downstream `ci-hub-smoke` pipeline #3100313274 is skipped and its `github_actions_request` job is manual. No actual GitHub Actions job logs were available. Therefore `tested`, `verified` and `completed` remain false.
- **Safety:** no historical release artifact, device-local data, Telegram session, Telegram message, production service, or user data was modified.


### Additional provider status update

GitLab commit status for implementation SHA `361f88de27d02d88b113ee728b59ada1e101584c` includes `github-actions/ci-hub-request: success` with [GitHub Actions run 45065897228](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/45065897228). This upgrades evidence from dispatch-only to **provider_result=success**. The run's individual job names/logs and test output are still unavailable here, so `jobs_verified` and `e2e_verified` remain unknown; Phase 00 stays implemented, not tested/verified/completed.
