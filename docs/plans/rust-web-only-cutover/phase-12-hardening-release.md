# Phase 12 — 보안·성능·운영·패키징·릴리스

**Kind:** foundation  
**Initial status:** not_started  
**Goal:** Harden the web-only product and prove supported headless server packages, observability, supply chain and upgrade paths.

## Repository surface

- `.github/workflows/**`
- `packaging/linux/**`
- `packaging/macos/**`
- `packaging/windows/**`
- `Dockerfile`
- `install.sh`
- `docs/operations/**`

## Acceptance criteria

- [ ] Linux Docker amd64/arm64 required
- [ ] macOS/Windows only headless server/CLI packages if tested. Checksums/provenance
- [ ] dependency audit
- [ ] crash/restart/load/perf
- [ ] redacted logs
- [ ] uninstall retention.

## Safety / negative cases

- [ ] No GPUI/native GUI or Android APK. No signing key/token exposure. Do not claim unsupported OS/browser support.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Rust fmt/clippy/test; Bun/browser E2E; Docker multiarch; headless host smoke where retained; license/CVE/SBOM.

## Rollback

Keep previous known-good artifacts and image digest; publishing requires separate approval.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
