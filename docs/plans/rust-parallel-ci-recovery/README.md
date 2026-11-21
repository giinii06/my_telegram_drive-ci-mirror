# Rust migration parallel CI — recovery and proof

> Project: `blond11qq/my_telegram_drive` (89276294)
> Target review branch: `ci/rust-parallel-workflow-repair-20261122`
> Base: `e4a98f880828d57cfcf2f2f4371b66f3ee269174` (Draft MR !21)
> User authorization: continue approved full Rust migration; no merge/deploy/mirror or runner changes
> Alchemy 2.2 phase-state snapshot: [phase-state.json](phase-state.json)
> Progress: **0 / 2 completed**

## Why this prerequisite matters

Actual GitLab project `ci_config_path` is `.gitlab/ci-hub-request.yml@blond11qq/ci-hub-smoke`; `.gitlab-ci.yml` in this branch is dormant. The Rust GitHub workflow `.github/workflows/rust-migration-parallel.yml` contains an unterminated quoted `go test` at line 49, unexpected `-count=1` at line 132 and duplicated `rust-contracts`, `legacy-go-tests`, `web-checks` job IDs. It is not valid reliable workflow evidence. GitLab `github-actions/ci-hub-request: success` is a provider callback from a *different* receiving workflow; it does not attest `cargo test` ran on this branch.

## Phase sequence

| Phase | Kind | Acceptance | State | Detail |
| --- | --- | --- | --- | --- |
| 01 | foundation | Deterministic parser/contract guard rejects broken YAML; repair six-job GitHub DAG with Go fixtures, Rust lint/tests and Bun checks | implemented (CI proof pending) | [Phase 01](phase-01-yaml-contract.md) |
| 02 | foundation | Independent GitHub run proves exact checked-out GitLab SHA and six real job categories; only then allow Rust phase verified | not_started | [Phase 02](phase-02-provider-evidence.md) |

## User-facing integration

Consumer: full Go→Rust Phase 07.c vault and subsequent Phase 05/06/08/11 user flows (login, encrypt/decrypt/upload/download). The CI repair is a prerequisite, not a working Telegram backend.

## Constraints and security

No change to GitLab `ci_config_path`, mirrors, runner quotas, project visibility, GitHub secrets or publication. The private GitLab repository must **not** be uploaded to an unreviewed public mirror. Existing Go web Docker publishing workflow remains untouched. Do not count a green dispatch/context as tests. If GitHub logs are inaccessible, leave Phase 02 blocked with recorded evidence and do not mark the full migration complete.
