# Rust migration parallel CI — recovery and proof

> Project: `blond11qq/my_telegram_drive` (89276294)
> Target review branch: `ci/rust-parallel-workflow-repair-20261122`
> Base: `e4a98f880828d57cfcf2f2f4371b66f3ee269174` (Draft MR !21)
> User authorization: continue approved full Rust migration; no merge/deploy/mirror or runner changes
> Alchemy 2.2 phase-state snapshot: [phase-state.json](phase-state.json)
> Progress: **0 / 3 completed**

## Why this prerequisite matters

Actual GitLab project `ci_config_path` is `.gitlab/ci-hub-request.yml@blond11qq/ci-hub-smoke`; `.gitlab-ci.yml` in this branch is dormant. The Rust GitHub workflow `.github/workflows/rust-migration-parallel.yml` contains an unterminated quoted `go test` at line 49, unexpected `-count=1` at line 132 and duplicated `rust-contracts`, `legacy-go-tests`, `web-checks` job IDs. It is not valid reliable workflow evidence. GitLab `github-actions/ci-hub-request: success` is a provider callback from a *different* receiving workflow; it does not attest `cargo test` ran on this branch.

## Phase sequence

| Phase | Kind | Acceptance | State | Detail |
| --- | --- | --- | --- | --- |
| 01 | foundation | Deterministic parser/contract guard rejects broken YAML; repair six-job GitHub DAG with Go fixtures, Rust lint/tests and Bun checks | tested locally (CI jobs unverified) | [Phase 01](phase-01-yaml-contract.md) |
| 02 | foundation | Independent GitHub run proves exact checked-out GitLab SHA and six real job categories; only then allow Rust phase verified | not_started | [Phase 02](phase-02-provider-evidence.md) |
| 03 | user_flow | Real Rust authenticated encrypted upload and download with correct failure/rollback behavior on isolated Telegram account | not_started | [Phase 03](phase-03-rust-operator-e2e.md) |

## User-facing integration

Consumer: full Go→Rust Phase 07.c vault and subsequent Phase 05/06/08/11 user flows (login, encrypt/decrypt/upload/download). The CI repair is a prerequisite, not a working Telegram backend.

## Constraints and security

No change to GitLab `ci_config_path`, mirrors, runner quotas, project visibility, GitHub secrets or publication. The private GitLab repository must **not** be uploaded to an unreviewed public mirror. Existing Go web Docker publishing workflow remains untouched. Do not count a green dispatch/context as tests. If GitHub logs are inaccessible, leave Phase 02 blocked with recorded evidence and do not mark the full migration complete.

## Alchemy v2.2 outcome linkage

Phase 01 and Phase 02 are **foundations**, and both point to Phase **03** as the explicitly planned consuming `user_flow`. This ensures `workflow_gates.py` can reject a workflow that delivers only checks/CI files but claims a finished product. Phase 03 is **not implemented, tested or promised as complete** by this CI recovery branch: the real Rust MTProto engine, Vault lifecycle, upload/download service, GPUI/server binaries and deployment remain in the broader [migration plan](../my-telegram-drive-rust-migration/README.md). This phase is only the cross-plan terminal integration acceptance owner; no user account, network or secret is accessed by CI recovery.

## Current exact evidence checkpoint

- Code `4017cab5c270b82d227c97b3ea459d42ae1454fc`: checksum-matched local Python contract 8/8 PASS, YAML parsed six unique jobs.
- External provider callback on code SHA: [GitLab #3098379726](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098379726) → [GitHub run 44959273388](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44959273388), `provider_result=success`, `jobs_verified=unknown`.
- Alchemy 2.2 offline state and outcome-gate consistency: 0 violations, next gate `VERIFY_REAL_CI_CHECKS` (Phase03 user-facing E2E explicitly not_started).
- The next **safe** action requires observable GitHub receiver job steps and checked-out SHA; do not infer them from the callback or change private/public mirror settings without authorization.

## Phase 01 extension — GH API real-run evidence verifier

A live GitLab bridge [pipeline 3098379726](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098379726) only has `dispatch_github_actions`, and downstream [ci-hub-smoke pipeline 3098379749](https://gitlab.com/blond11qq/ci-hub-smoke/-/pipelines/3098379749) was **skipped**. The separate GitHub success status does not identify executed Rust/Go/Bun job results. Public GitHub run/job URLs and API are inaccessible from this chat's web/container environment, and repo is private; no evidence of mirrored source ref. We must not change runner or mirror settings as a workaround.

To prevent this attribution mistake in future, Phase 01's existing source-check repair extends to:
- explicitly name mandatory Rust, Go and Bun job steps and add `Verify checkout SHA` after `actions/checkout` in **every** job;
- add a dependency-free, read-only GitHub REST verifier `scripts/verify_rust_gha_run.py`. Given `--repo`, `--run-id`, `--sha`, it must check real GitHub API `run.name`, `run.path`, `run.head_sha`, `run.conclusion`, `run.repository.full_name`, all six job names (including parallel independence/dependencies from source) and per-job mandatory successful steps; it rejects absent jobs, wrong SHA, wrong workflow, a provider-only callback, incomplete/duplicate/failed jobs and API failures. No GH token or GitLab session is passed by default.
- add `scripts/test_rust_gha_run.py` with deterministic fake job summaries to demonstrate RED then GREEN locally. Do not count synthetic test passes as a real run.

After code+test commits, Phase 01 remains `tested` only until the *actual GitHub REST job responses* prove `jobs_verified`. Phase 02 remains not_started/blocked pending real provider visibility. This is exactly the existing approved CI proof scope, no new user decision or deployment.

## Phase 01 extension — latest code-SHA local GREEN (2026-11-22)

- Exact code candidate: `5aeb52a881616eb4f3eb1468198a01aece56be6d`. This is **not** the later docs-only checkpoint commit.
- GitLab raw files independently matched local SHA-256: `scripts/verify_rust_gha_run.py` `97a63ff594dd252481f423ce26e9d5ca760d6775f349065ec12340fa901efb3d`; `scripts/test_rust_gha_run.py` `d7526caf1e77728b24c0234001d6e1f1b97bab7cc62f39ea2d91cc6108b30930`; `scripts/check_rust_ci_workflow.py` `f2fb025b33b5486653058143375a7d659486ca62761fb31568eb1fa5a9e94b6d`; `scripts/test_rust_ci_workflow.py` `2bf4bdb545619368c67e7fb3c05fb764e9dd76b6a945de1ee9c562dcb1e32a38`; workflow YAML `072ce6f3d37663465b2a994511ac08028d50a3ea7fcb2b49b3c939bd8380ff74`.
- Observed real local commands in temporary read-only source copy: `python3 scripts/check_rust_ci_workflow.py` PASS; `python3 scripts/test_rust_ci_workflow.py` **12/12 PASS**; `python3 scripts/test_rust_gha_run.py` **16/16 PASS**; `python3 -m compileall -q scripts` PASS; PyYAML BaseLoader syntax PASS, six unique GitHub jobs, **every step has run or uses**, SHA attestation in each. No Rust/Go/Bun repo checkout or test execution claimed.
- RED found while adding named steps: the artifact upload `- name` and `- uses` were separate YAML steps. Corrected in `5aeb52a881616eb4f3eb1468198a01aece56be6d`; guard plus test `test_name_only_step_is_rejected` now rejects these invalid actions. Source YAML semantic check passed after the correction.
- Real `python3 scripts/verify_rust_gha_run.py --repo giinii06/my_telegram_drive-ci-mirror --run-id 44959273388 --sha 4017cab5c270b82d227c97b3ea459d42ae1454fc` exited **1** with `result=blocked` and `GitHub API request unavailable (network/permissions/status)`. This is correct fail-closed behavior, not a GitHub test failure.
- [GitLab API pipeline 3098443401](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098443401) created on exact new code SHA `5aeb52a881616eb4f3eb1468198a01aece56be6d`; GitLab bridge downstream pipeline 3098443435 was `skipped` because it only provides a request/dispatch bridge. `github-actions/ci-hub-request` had only `pending` status at this checkpoint with [GitHub run 44962575820](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44962575820). Never infer Rust job success from that callback.
- **Alchemy v2.2 Phase 01 remains tested, Phase 02 and future Phase 03 remain not_started. No jobs_verified, no E2E proof and no approval for changing mirror/secrets/runner.**
