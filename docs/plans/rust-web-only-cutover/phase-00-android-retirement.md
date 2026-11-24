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

### 2026-11-23 — APK regression contract hardening (same Phase 00)

- **RED fixture commit:** `7bd21aaad9a611d55fcd2700449d74db42b3b405`. On a local reduced-source fixture, two adversarial cases failed as expected: renamed `mobile-release` running `gradlew assembleRelease` bypassed the old Android check, and four installer names in comments falsely satisfied the old retention check.
- **GREEN implementation commit:** `fea5200033b62b8b6bfe9a350073ac254ccea4e6`. Hardened the static Android job/build path checks, anchored real installer job declarations, and included the adversarial unit-test file in the existing GitHub static-cutover workflow.
- **Local run:** `python3 -m unittest -v scripts/test_web_only_support.py scripts/test_web_only_support_robustness.py` — 6/6 PASS on a reduced local fixture representing current source interfaces; `python3 -m py_compile` passed. **Not** an actual full-checkout run, not Rust/Go/Bun test evidence.
- **Live GitLab source audit:** `.github/workflows/installers.yml` at code SHA above still declares `linux-deb`, `macos-dmg`, `windows-exe`, and `windows-backup-exe` as actual jobs and has no APK/Gradle build marker.
- **CI:** [GitLab pipeline #3101014028](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3101014028), exact code SHA `fea5200033b62b8b6bfe9a350073ac254ccea4e6`, reports `dispatch_github_actions=success`. At inspection, **no matching provider callback or GitHub per-job logs** were accessible. `provider_result=unknown`, `jobs_verified=unknown`, `e2e_verified=unknown`.
- **Two-lens self-review:** requirements tests directly cover the two discovered false negatives/positives; security/quality confirms no app runtime, secrets, installers or release artifacts were modified, but real CI execution is still unverified. Both reviews were self-review, not independent.
- **Gate:** Phase 00 remains `implemented` only. Do not advance to Phase 01 or call Phase 00 completed without actual full-repository and provider job evidence (or a separately approved scoped waiver, which would not count as passed).

### All-workflow regression coverage

- RED commit `b9f3b0a8d7f1b6690229e7f3795de465ed7d68ef`: local exact-source adversarial Python suite 1 expected failure / 2 pass; APK build in another workflow was accepted incorrectly.
- GREEN commit `62610d3bda9c42f74c3a375d04b5a51c38a21bc7`: scan all .yml/.yaml GitHub workflow files; require existing installers.yml. Local adversarial suite 3/3 passed and py_compile passed; both executed Python source blob IDs match GitLab. No full checkout or Rust/Bun tests executed.
- Two-lens self-review: actual diff covers the observed false negative, retains the existing installer check and does not touch production data, releases or runtime. External job evidence remains unavailable.
- [Pipeline #3106069231](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3106069231) exact candidate: dispatch success, provider result/jobs/E2E unknown at inspection. Previous candidate #3101014028 later returned provider success (run 45103871574), not per-job proof.
- Phase 00 remains implemented; Phase 01 is not advanced. No merge/deploy or provider changes.
