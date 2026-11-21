# Phase 01 — reconstruct Rust parallel workflow and protect its contract

**Status:** not_started
**Implemented:** no
**Tested:** no
**Committed:** no
**Gate version:** 2.2
**Kind:** foundation
**Consumer user flow:** encrypted drive upload/download and full Rust backend end-to-end flows
**Base SHA:** `e4a98f880828d57cfcf2f2f4371b66f3ee269174`

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
