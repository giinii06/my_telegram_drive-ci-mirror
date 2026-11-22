# Phase V.1 — Private source local Rust build verifier (no public history publishing)

**Authority:** private GitLab project 89276294, existing Draft MR !24, source `refactor/rust-only-cutover-20261122`, starting HEAD `c102be7951e6125629150d527eaa51b63b2a937f`. Full Go→Rust feature parity remains approved and unfinished. This is a verification *foundation* and cannot complete a user-facing file drive.

## Why this gates further implementation
- The effective GitLab-to-GitHub bridge had `github-actions/ci-hub-request=failed` with description `The requested Git history did not pass the publication scan` for pipeline 3099037733; the downstream GitLab child is `skipped/manual`, not a Cargo test runner.
- Publishing a private historical Git object or bypassing the scan is outside current authorization. Current execution sandbox has no cargo/rustc/Docker and no GitLab-host checkout; local toolchain running on operator Mac/Linux is a privacy-preserving alternative, not a new network-side provider.

## User-visible acceptance
- A **single opt-in shell command** verifies the existing private GitLab checkout on the operator's own Mac or Linux host without publishing Git history: `bash scripts/verify_mtproto_local.sh`.
- Refuses missing `cargo +1.92.0` and `rustc`, dirty Git checkout or wrong user-supplied expected SHA; prints exact checked-out 40-char SHA. Fails closed if nested `apps/tdrive-mtproto/Cargo.lock` is absent, directs operator to run `cargo +1.92.0 generate-lockfile --manifest-path apps/tdrive-mtproto/Cargo.toml` **manually, audit and commit it** before re-running. Never invent hashes.
- Runs **actual** Rust root and nested fmt checks; nested locked clippy/test/release build; Docker build and no-credential `--help` only when Docker exists. Never logs in, never executes `watch`/`upload` or touches Telegram account / Go v16 DB.
- Credentials are not required, no private Git content is copied to public mirrors. Logs contain exact source SHA, commands and exit codes; the run itself must be executed on a trusted authorized checkout. Script does not claim CI provider attestation.
- Add a dependency-free Python static contract for fail-closed semantics and shell `bash -n` syntax check, executable in the current sandbox using only reviewed script content. These checks verify shell mechanics, NOT Rust compilation.

## Negative and privacy acceptance
- Missing lockfile, unavailable toolchain, dirty checkout, wrong SHA or a failing Cargo command -> nonzero exit. No auto-generation of lockfile or retry while unsafe.
- Docker optional controlled by `TDRIVE_CHECK_DOCKER=1`, and then must be installed; never pass TG_API_ID/TG_API_HASH or mount real session DB into CI smoke.
- No changes to project CI provider, GitLab mirror/runner/secret settings, GitHub plugin, repository visibility, pipeline publication scanner, deployment, merge, or Go production data.

## Gate ownership and handoff
- This phase belongs to CI/verification prerequisite for Phase A/Phase B (Go v16 projection) and Phase C (MTProto/vault), but **does not waive** their existing failed/unknown provider tests. Native exact-source build/test and isolated Telegram E2E still required before production readiness.
- After actual operator run, capture stdout/exit + exact SHA without secrets; review any Cargo errors, repair code and rerun. Only then consider normal phase advancement with two-lens review and real job evidence.
