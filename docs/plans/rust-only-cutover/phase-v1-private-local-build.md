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

## 2026-11-22 source-exact local verification gate

**Implemented source SHA:** `ab4d23dd2d6fd933447fbe9fc07f13af145af962`. `scripts/verify_mtproto_local.sh` + `scripts/test_verify_mtproto_local.py`. Test-only GitLab SHA `d6ccf4485ccbf66c51f701035c3efdcdcb57fcbe`; plan SHA `4e9fe7affcd0f1f2d2a845447a61b43a920c8d19`.

### Executed local commands and identity

Local Python/Ubuntu container (synthetic isolated Git repositories, fake `cargo`/`rustc`/`docker`, **no private source checkout**):

- `bash -n /mnt/data/mtproto-v1/scripts/verify_mtproto_local.sh` => **exit 0**.
- `python3 -m unittest discover -s /mnt/data/mtproto-v1/scripts -p 'test_verify_mtproto_local.py' -v` => **5/5 PASS**, 0.14 sec on final run.
- Test-only first attempt => **5 failures/errors** because intended script did not yet exist; this is not a behavioral Rust RED verification. First combined implementation run => **4/5 PASS**, one fail because fake cargo did not produce an executable; adjusted the test fixture to create an executable stub. Final => 5/5 PASS.
- GitLab `get_repository_file` returned matching SHA-256 and byte length for **exact checked-in files**: shell `b6d9bc875d9e66dcc311d0c70d2cee30830467ddd1e4c80e1cfa10144d0ad967` (3090 bytes); test `4619fbc24cb34336f6b0fcce9b777c65899de7a8da000b7ba1281019e07c784e` (5448 bytes). Exact same checksummed bytes were locally executed.

### What was verified and what remains blocked

- Verified **runner contract**: wrong exact SHA/dirty tree/missing nested Cargo.lock exit nonzero; dry-run fake Rust fmt/clippy/test/release checks in order; an injected clippy failure aborts before tests/Docker; opt-in Docker uses only dummy variables and `--network none` CLI smoke; no real Telegram credentials requested.
- **NOT executed:** Rust `cargo fmt`, Clippy, actual `cargo test` or `cargo build`, true Docker build, actual Grammers API check, native macOS run, Telegram login/upload/download, GPUI, WebDAV, HLS, Go-17-operation transaction replay, vault and migration E2E. This sandbox has no rustc/cargo/Docker and no authenticated private checkout. **The nested `apps/tdrive-mtproto/Cargo.lock` is absent in current GitLab HEAD.** Do not pretend to pass any of those gates.
- Current source GitLab→public GitHub CI bridge publication scan is still rejected: see `ci-publication-scan-blocker.md`. No new pipeline was launched on this docs/script-only verification checkpoint, because the effective provider cannot run private source and a redundant pipeline would not compile Rust. `dispatch=not requested for this candidate`; `provider_result=unknown`; `jobs_verified=unknown`; `e2e_verified=unknown`.

### Private operator action once a trusted checkout is available

From the operator's *private* GitLab checkout on macOS/Linux:

```sh
cargo +1.92.0 generate-lockfile --manifest-path apps/tdrive-mtproto/Cargo.toml
# Review changed dependency tree, provenance, Cargo.lock and ensure no secrets.
git add apps/tdrive-mtproto/Cargo.lock
git commit -m 'build(rust): resolve reviewed nested MTProto dependencies'
bash scripts/verify_mtproto_local.sh
# Optional local Docker checks, on the private host only:
TDRIVE_CHECK_DOCKER=1 bash scripts/verify_mtproto_local.sh
```

Do not run this against production session or user data. No automated external Git publication or secret disclosure, no merge/deploy. Treat any actual Cargo error as a real blocker to be repaired on that exact reviewed SHA. This is a CI foundation consumed later by Rust user-account E2E and full 17-op Go-parity migration.
