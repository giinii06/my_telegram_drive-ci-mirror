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

## CI run evidence RED (2026-11-22 continuation)

Before this change, the actual GitLab source for all six GitHub jobs lacked an explicit named `Verify checkout SHA` step (the legacy job alone had a differently named SHA check). The pinned-source structural RED check observed 0/6 of the required step name. This change makes GitHub run-job API results attest to the step in **every** job, alongside named Go oracle, Rust fmt/clippy/test and Bun lint/test commands. Failure to fetch real GitHub job results remains BLOCKED. The plain local script `scripts/verify_rust_gha_run.py` does not fabricate provider runs.

## Phase 01 extension — latest code-SHA local GREEN (2026-11-22)

- Exact code candidate: `5aeb52a881616eb4f3eb1468198a01aece56be6d`. This is **not** the later docs-only checkpoint commit.
- GitLab raw files independently matched local SHA-256: `scripts/verify_rust_gha_run.py` `97a63ff594dd252481f423ce26e9d5ca760d6775f349065ec12340fa901efb3d`; `scripts/test_rust_gha_run.py` `d7526caf1e77728b24c0234001d6e1f1b97bab7cc62f39ea2d91cc6108b30930`; `scripts/check_rust_ci_workflow.py` `f2fb025b33b5486653058143375a7d659486ca62761fb31568eb1fa5a9e94b6d`; `scripts/test_rust_ci_workflow.py` `2bf4bdb545619368c67e7fb3c05fb764e9dd76b6a945de1ee9c562dcb1e32a38`; workflow YAML `072ce6f3d37663465b2a994511ac08028d50a3ea7fcb2b49b3c939bd8380ff74`.
- Observed real local commands in temporary read-only source copy: `python3 scripts/check_rust_ci_workflow.py` PASS; `python3 scripts/test_rust_ci_workflow.py` **12/12 PASS**; `python3 scripts/test_rust_gha_run.py` **16/16 PASS**; `python3 -m compileall -q scripts` PASS; PyYAML BaseLoader syntax PASS, six unique GitHub jobs, **every step has run or uses**, SHA attestation in each. No Rust/Go/Bun repo checkout or test execution claimed.
- RED found while adding named steps: the artifact upload `- name` and `- uses` were separate YAML steps. Corrected in `5aeb52a881616eb4f3eb1468198a01aece56be6d`; guard plus test `test_name_only_step_is_rejected` now rejects these invalid actions. Source YAML semantic check passed after the correction.
- Real `python3 scripts/verify_rust_gha_run.py --repo giinii06/my_telegram_drive-ci-mirror --run-id 44959273388 --sha 4017cab5c270b82d227c97b3ea459d42ae1454fc` exited **1** with `result=blocked` and `GitHub API request unavailable (network/permissions/status)`. This is correct fail-closed behavior, not a GitHub test failure.
- [GitLab API pipeline 3098443401](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098443401) created on exact new code SHA `5aeb52a881616eb4f3eb1468198a01aece56be6d`; GitLab bridge downstream pipeline 3098443435 was `skipped` because it only provides a request/dispatch bridge. `github-actions/ci-hub-request` had only `pending` status at this checkpoint with [GitHub run 44962575820](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44962575820). Never infer Rust job success from that callback.
- **Alchemy v2.2 Phase 01 remains tested, Phase 02 and future Phase 03 remain not_started. No jobs_verified, no E2E proof and no approval for changing mirror/secrets/runner.**

## Continuation — bind each GitHub job to the requested run and source SHA

The GitHub REST workflow-job schema supplies `run_id`, `head_sha` and a numeric job `id`. Prior verifier checks run-level identity and named steps, but omits *job-level* provenance. An unrelated successful job record must not satisfy the six-job proof even when it has the expected name and successful step names.

**RED acceptance:** Synthetic successful run with one job's `run_id` changed, its `head_sha` changed, a missing job identifier, or a duplicate numeric job identifier must be rejected; the existing normal six-job fixture must remain accepted. These are offline regression tests only.

**GREEN implementation:** In `scripts/verify_rust_gha_run.py`, bind each job's `run_id` to `run.id` and its `head_sha` to requested exact source `sha`; require unique positive numeric GitHub job IDs (excluding bool). Maintain current fail-closed JSON error reporting and no secrets. Preserve all existing CLI flags and workflow job contract names.

**Evidence gate:** commit RED tests first, observe the failing test on exactly the saved scripts when a local executor is available, then implement and run existing/added Python tests. Matching-SHA real GitHub `rust-contracts`, Go fixtures and Bun jobs remain unverified until actual provider response data is accessible. No CI provider, secrets, visibility, or mirror mutations are authorized by this continuation.

## 2026-11-22 continuation checkpoint — job provenance guard

- Approved existing design and Phase 01 resumed on `ci/rust-parallel-workflow-repair-20261122`; plan-first commit `2ac51098e2a7b921ac763ab5706a94eca64e4126`.
- RED regression-only commit `10286b01b9e73e64a998b52f71c4d51e0e3f7840`: five cases reject wrong job run ID, wrong job head SHA, missing job ID, duplicate numeric IDs, bool IDs. Baseline valid fixture remains accepted.
- Local Python **reproduction** of the source validator (not a byte-exact checkout of the private GitLab repository) had 5/6 tests FAIL before fix, then 6/6 PASS after applying the same provenance conditions; `python3 -m py_compile` of the temporary reproduction succeeded. Do **not** call these tests exact-repository CI results.
- GREEN code commit `d2cbd9f1eff29e21401cfc7d77cbcb54d68e2158` enforces positive integer GitHub run ID, unique positive integer job IDs and per-job `run_id`/`head_sha` identity before accepting job steps. Inspected the actual GitLab commit diff, source and test file after commit.
- Exact-SHA [GitLab pipeline #3098797918](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098797918): `sha=d2cbd9f1eff29e21401cfc7d77cbcb54d68e2158`, GitLab pipeline `success`, `dispatch_github_actions=success`, **no GitLab CI job results and no matching `github-actions/ci-hub-request` status yet observed at this checkpoint**. This is `dispatch` only. No verified GitHub run, six job logs, Cargo/Go/Bun execution or real Telegram E2E.
- The [new two-lens sequential self-review](reviews/phase-01-job-provenance.json) checks the same actual code SHA; **not** independent review. New code SHA must not inherit prior `tested` evidence from `5aeb52...` without an exact-file suite or actual GitHub jobs. Phase 01 remains not verified/completed, and Phase 02/03 remain blocked/not_started. No merge, deployment, mirror, runner, secrets or project visibility changes were made.

## Phase 01 extension — reject comment-only CI fixture environment declarations

**Root cause observed at existing code SHA `3f8a8076aadbabebf0bd3eafd1940481c3deece7`:** `scripts/check_rust_ci_workflow.py` checks every `NEEDED_ENV` name anywhere in the YAML text, rather than requiring actual `$GITHUB_ENV` assignment in the producing/consuming jobs. The current Python regression fixture puts all these names in trailing `#` comments, which passes that weak requirement. That can falsely certify fixture plumbing after removing live environment assignments.

**Planned test-first scope:** keep the six-job contract, path and CLI unchanged. Add regression tests where a fixture exporter output environment assignment is absent but the name survives in a YAML comment, and where a Rust import path environment assignment is absent but its name survives in comments. Those tests must fail under the original checker. Then require exact `echo "KEY=$GITHUB_WORKSPACE/..." >> "$GITHUB_ENV"` assignments in `go-fixtures` for `TDRIVE_COMPAT_{FIXTURE,TDE1,NAME,FOLDER_LISTING,REPLAY,DIGEST}_OUT`, and in `rust-contracts` for `TDRIVE_GO_{SQLITE_FIXTURE,SQLITE_LISTING_FIXTURE,SQLITE_REPLAY_FIXTURE,SQLITE_DIGEST_FIXTURE,TDE1_FIXTURES_DIR,NAME_FIXTURE}`. The tests' valid fixture must contain actual assignment lines, not only comments. Negative cases must reject removal even if an adversarial comment preserves the same text. Add no dependencies or provider permissions.

**Verification contract:** before declaring GREEN, execute `python3 scripts/check_rust_ci_workflow.py` and `python3 scripts/test_rust_ci_workflow.py` against exact source if the executor can access the files. Then request a matching 40-char GitLab code SHA pipeline and inspect actual GitHub jobs. A dispatcher callback remains insufficient. If the remote Rust job proof remains inaccessible, Phase 01 stays `tested` at most, Phase 02 `blocked`, Phase 03 not started. No production data, users, mirror, secrets, runner, merging or deployment.
