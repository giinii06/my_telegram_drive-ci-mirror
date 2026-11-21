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
