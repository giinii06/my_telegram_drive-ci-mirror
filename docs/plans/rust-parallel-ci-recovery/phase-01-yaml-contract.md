# Phase 01 — reconstruct Rust parallel workflow and protect its contract

**Status:** tested
**Implemented:** yes
**Tested:** yes
**Committed:** yes
**Gate version:** 2.2
**Kind:** foundation
**Consumer user flow:** encrypted drive upload/download and full Rust backend end-to-end flows
**Base SHA:** `e4a98f880828d57cfcf2f2f4371b66f3ee269174`
**Candidate SHA (latest code):** `4017cab5c270b82d227c97b3ea459d42ae1454fc`

## Root cause and acceptance

At the pinned base, `.github/workflows/rust-migration-parallel.yml` has:
- line 49: `go test` expression missing closing quote / `-count=1`
- line 132: orphan `-count=1`
- line 133: duplicate append of the name exporter with an unterminated quote
- duplicated top-level jobs starting `rust-contracts` line 154, `legacy-go-tests` line 183, `web-checks` line 202

**Positive:** exactly six independent/aligned jobs `legacy-contracts`, `rust-style`, `go-fixtures`, `rust-contracts`, `legacy-go-tests`, `web-checks`, with only `rust-contracts.needs=go-fixtures`. Go exporters for v16 SQLite, TDE1, seeded TDE1 and filename goldens run as real complete commands, required files move through artifact to Rust tests; no production data, secrets or release job.

**Negative:** reject duplicate jobs, truncated Go commands, missing fixture exporter/outputs, missing Rust lint/tests, loosened permissions, accidental publish or private data upload. A malformed workflow MUST fail the repo-local contract checker before merge.

## Test-first implementation steps

1. Record existing malformed YAML and run read-only failure detection on current remote content (RED).
2. Add dependency-free `scripts/check-rust-ci-workflow.py` with strict known workflow checks and self-tests for malformed fixture lines and duplicate jobs; do not falsely call the lexical checker a complete YAML parser.
3. Repair YAML without modifying the GitHub receiver or GitLab CI provider. Run `python scripts/check-rust-ci-workflow.py` plus local PyYAML syntax validation if available; actual GitHub native parser only when the branch is delivered.
4. Candidate commit, exact-SHA real CI/provider verification; separate requirements and quality/security reviews; record checkpoint with precise evidence.
5. If provider job execution cannot be observed, do NOT mark Phase 01 `verified/completed`; keep implemented or blocked.

## Verification

- Local: `python3 scripts/check-rust-ci-workflow.py`, self-test negative fixtures; PyYAML parse with duplicate-key-aware loader where available.
- GitHub: same GitLab SHA, `legacy-contracts`, `rust-style`, `go-fixtures`, `rust-contracts`, `legacy-go-tests`, `web-checks` all executed and passed, rust job must actually run `cargo clippy` and `cargo test`.
- Evidence class: `provider_result` until actual job checkout/logs are inspected; no runtime proof from a synthetic CI rule file alone.
- Precommit executor: local shell exists only for Python checker, no Rust toolchain or authorized private repo checkout. Rust compilation not locally executed.
- No merge/deploy; reviewer traces and decisions recorded in `docs/plans/rust-parallel-ci-recovery/`.

## Observed failure and root cause

Real read-only source check on `60ffec1cfa5e77a973021de27ba99d1d3d716f85`: 12 top-level job declarations instead of six (three repeated jobs), 2 orphan `-count=1` lines, incomplete seeded/name Go fixture commands. The previously-used JavaScript `String.replace(old, replacementString)` expanded the Go regex suffix `$'` as a **special replacement token meaning the remainder of the source string**, thereby duplicating the rest of the workflow. The corrective transformation uses a function callback `replace(old, () => replacement)` so `$'` is literal. This is a *reproduced root cause* (5,805-byte first segment ballooned to 13,595 bytes under naive replacement in a safe read-only experiment).

RED contract cases were committed independently as `60ffec1c`. Existing broken source fails those predicates. No standalone Python RED process was observed on the old private checkout, only the source-backed read-only structural check.

## Candidate remediation

Rebuild six intended job sections, restore both Go fixture exporter calls, require Rust to check every produced fixture, and invoke the new guard/self-tests in `legacy-contracts`. This is only a candidate; GitHub's native workflow parser and individual job checks still need independent execution/checkout-SHA proof.

## Alchemy v2.2 checkpoint (unverified implementation)

- Source candidate: `c51c6d3419ab96c018fbbd93c2589a4c8bd07e40`; GitLab pipeline [#3098370541](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098370541). External provider status is **not** independent job execution proof.
- Read-only six-job source structure and full Go oracle string presence: observed PASS on pinned GitLab raw content. **Python checker process, YAML parser and Cargo/Go/Bun jobs: not executed/observed locally.**
- Requirements and security review: two separately recorded, sequential **self-review** lenses at [reviews/phase-01.json](reviews/phase-01.json). This is not an independent reviewer.
- Machine state: `implemented` only. `tested`, `verified` and `completed` remain false until actual jobs and checked-out SHA are inspected.

## Exact-SHA external provider status at checkpoint

- Candidate code SHA: `c51c6d3419ab96c018fbbd93c2589a4c8bd07e40`.
- [GitLab pipeline #3098370541](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098370541), status `success`, GitLab-only job list empty: no GitLab Runner Rust tests were executed.
- Matching GitLab external `github-actions/ci-hub-request: success` points to [GitHub Actions run 44958713063](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44958713063). **Only provider_result**, not `jobs_verified`; actual checked-out SHA and six job logs not accessible via connected GitLab or web/browser in this session.
- JS read-only structure check of actual candidate YAML passed; Python workflow validator/pytest or Rust/Go/Bun were not executed by a local authorized private-repo checkout. GitHub-native YAML parsing and run of this workflow specifically remain **UNVERIFIED**.
- Code review at [reviews/phase-01.json](reviews/phase-01.json) is sequential self-review, not an independent reviewer. No claim of product end-to-end completion.
- Phase State Machine remains `implemented` (not tested/verified/completed); no waiver is granted.

## Local exact-file GREEN test evidence (Phase01 tested, NOT verified)

After the initial `c51c6d3` candidate, the fixture guard accidentally checked `check-rust-ci-workflow.py` while the real workflow called `check_rust_ci_workflow.py`. The original `scripts/check_rust_ci_workflow.py` was reconstructed in a temporary nonpublic local workspace and its SHA-256 matched GitLab `ab6450a6866c8202f854d59acec41c92919c2be409e0749792205adb65cbfdb3`. Executing `python3 scripts/check_rust_ci_workflow.py` observed the expected **RED** `FAIL workflow must enforce its own contract check`. Fix `2f4b26e0fdc354b498f79048e3d7936f166a54d0` aligned that path. The test fixture had the same old hyphen path; fix `4017cab5c270b82d227c97b3ea459d42ae1454fc` aligned it.

The exact three checked GitLab-file SHA-256s after fixes, independently verified against a local temporary copy:
- workflow: `d07cd721a95e8fb32a486117f8922a30794e173797a2de55d878934a36799915` (6,037 bytes)
- `scripts/check_rust_ci_workflow.py`: `9a4a3b567a9051c39a9cd7c18580456a95a72dbef2e158c47d0f6b7cb5322440` (6,445 bytes)
- `scripts/test_rust_ci_workflow.py`: `780d3aa7be0195a09b42f46763a05c499353dbf761cf6d4ddb320c6f71824567` (4,637 bytes)

Observed local commands: `python3 scripts/check_rust_ci_workflow.py` **PASS**, `python3 scripts/test_rust_ci_workflow.py` **8/8 PASS**, `python3 -m py_compile scripts/check_rust_ci_workflow.py` **PASS**, and `yaml.load(workflow, Loader=yaml.BaseLoader)` **PASS** with exactly six GitHub jobs. This proves local structural/parsing behavior of these exact bytes, **not GitHub Actions job execution**. Rust toolchain unavailable in this container and private GitLab source not checked out locally. [New same-code SHA provider pipeline #3098379726](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098379726) remains separate and must not be conflated with local tests.

Alchemy Phase State Machine: `tested`, *not* `verified` or `completed`; real GitHub job/checkout SHA proof remains mandatory before Phase 02. No waiver.

## Observed RED case classification

The original malformed YAML failed source-backed structural RED. In addition, the exact GitLab checker copy at the initial candidate `c51c6d3` was locally executed and **failed** with `FAIL workflow must enforce its own contract check` because its expected filename had hyphens instead of underscores. This is genuine observed RED for that regression. After commits `2f4b26e0` and `4017cab5`, identical-hash source copies produced PASS and eight unittest results. The v2.2 state marks `red_verified=true` for this specific checker contract, not for all downstream Rust/Go tests.

## Latest provider recheck for final code candidate

- **Code SHA:** `4017cab5c270b82d227c97b3ea459d42ae1454fc` (not the newer docs checkpoint).
- **GitLab pipeline:** [#3098379726](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098379726). `dispatch_github_actions=success` and external `github-actions/ci-hub-request=success` with [GitHub run 44959273388](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44959273388). This is `provider_result` only; no GitHub checked-out SHA or per-category test job logs are visible through the available connector/web/browser.
- **Actually run locally on SHA256-matched bytes:** `python3 scripts/check_rust_ci_workflow.py` PASS; `python3 scripts/test_rust_ci_workflow.py` 8/8 PASS; `python3 -m py_compile scripts/check_rust_ci_workflow.py` PASS; PyYAML `yaml.BaseLoader` parsed `jobs` with six unique IDs. `cargo fmt/clippy/test`, Go exporters and Bun tests **NOT observed**.
- The three Phase state records (Phase01 tested, Phase02/Phase03 not_started) were validated with `Alchemy scripts/phase_state.py` and `scripts/workflow_gates.py`: both returned **zero violations** and `next_gate=VERIFY_REAL_CI_CHECKS`. These checks were executed on a local JSON reconstruction of the exact GitLab state values. They are offline consistency tests, not independent CI or E2E proof.
- No fresh request to change GitHub receiver workflows, private mirrors/secrets or GitLab runner settings was authorized. No waiver granted; gate remains open.

### Additional evidence-proof invariant (planned before implementation)

The workflow currently checks `git rev-parse HEAD == GITHUB_SHA` only in `legacy-contracts`, not all jobs. To prevent false status attribution, add an explicit same-SHA step in every job and programmatically verify run/job evidence against the **requested GitLab commit**. Read-only GH API calls are permitted but any repository/project visibility, secrets, mirror or runner settings remain unchanged. Nonpublic GitHub runs without job access must return UNKNOWN/BLOCKED, never PASS.
