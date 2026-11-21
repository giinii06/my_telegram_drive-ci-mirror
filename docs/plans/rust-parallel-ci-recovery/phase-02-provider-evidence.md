# Phase 02 — provider-level exact-SHA job proof

**Status:** not_started
**Implemented:** no
**Tested:** no
**Committed:** no
**Gate version:** 2.2
**Kind:** foundation
**Consumer user flow:** Phase 07.c vault and completed Rust headless/GPUI workflows

## Preconditions

Phase 01 exact source SHA is verified at `jobs_verified` level. GitLab `ci_config_path` and selected GitHub Actions receiver are re-read immediately before the check. Do not assume GitLab Runner.

## Acceptance

1. GitLab `create_pipeline` returns the **exact candidate SHA**, and `list_commit_statuses` for that SHA+pipeline ID reports provider status and run URL.
2. Receiving GitHub workflow and **checked-out GitLab source SHA** match the same candidate. The event's default-branch SHA alone is not proof.
3. Inspect actual logs/steps (or previously independently audited per-category signed reporter) for Rust fmt+clippy+test, Go oracle generation+Go regression, Bun lint+unit and named artifact transfer. Distinguish x86_64 from macOS/ARM64.
4. Preserve per-category evidence, job URLs and command output; run two-lens review; then phase-state transitions one by one.
5. If GitHub actions access/job logs are unavailable, Phase 02 remains **blocked**, never converted to verified on a generic `github-actions/ci-hub-request: success`.

## Security / out-of-scope

Do not change mirror, CI secrets, runner or repo visibility to satisfy CI. No production Telegram credentials, deployment or live user data. This is a verification gate, not the last Rust implementation Phase.

## Observation from Phase 01 candidate (does not clear this phase)

A generic external success callback exists for GitLab source `c51c6d3419ab96c018fbbd93c2589a4c8bd07e40` at pipeline 3098370541, GitHub run 44958713063. That status has no per-category `alchemy/format`, `alchemy/lint`, `alchemy/test`, `alchemy/build` contexts. The effective GitLab CI is the external ci-hub dispatcher and the GitLab Runner job list is empty. Requirement 02.1–02.4 **not satisfied**; no claim of cargo, Go or Bun tests passing. To clear, inspect the actual GitHub receiving workflow and checked-out SHA plus step/job result evidence, or adopt a separately approved category-status reporting bridge without revealing private source to the public mirror. No repo visibility/mirroring/secrets or runner settings changed.

## Consuming user flow

This foundation explicitly points to the never-started **Phase 03** operator encrypted upload/download E2E gate. Phase 02 cannot be marked jobs-verified solely because the provider sends `github-actions/ci-hub-request: success`. Absence of remote job logs remains a security/verification blocker for the larger Rust migration.

## Latest code SHA request result

For `4017cab5c270b82d227c97b3ea459d42ae1454fc`, GitLab pipeline [#3098379726](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098379726) received external GitHub Actions success callback [run 44959273388](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44959273388). Provider callback does not establish that the **repaired** `rust-migration-parallel.yml` workflow ran. Remote GitHub job-level evidence is inaccessible, so Phase02 remains not_started/blocked on external visibility and cannot clear the Phase01 `verified` transition. This does not authorize further dependent Vault implementation as a verified migration Phase.

### GitHub REST verifier contract (planned)

`scripts/verify_rust_gha_run.py --repo giinii06/my_telegram_drive-ci-mirror --run-id <actual-run> --sha <40hex>` must inspect **GitHub's own** runs/jobs JSON, not GitLab's generic status. The exact workflow path `.github/workflows/rust-migration-parallel.yml` and complete six job steps/checkout SHA checks must match; otherwise fail closed. Runtime access from this ChatGPT environment failed (GitHub API DNS, web cache). A local fake fixture cannot claim provider jobs.
