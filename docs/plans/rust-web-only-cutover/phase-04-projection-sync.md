# Phase 04 — 17-op TDX1 projection·history/live sync

**Kind:** user_flow  
**Initial status:** not_started  
**Goal:** Rust rebuilds file/folder state from Telegram operation messages and continuously converges with remote history.

## Repository surface

- `crates/tdrive-projection/**`
- `crates/tdrive-storage/**`
- `crates/tdrive-telegram/**`
- `crates/tdrive-sync/**`
- `apps/tdrive-server/**`
- `tests/compatibility/tdx1/**`

## Acceptance criteria

- [ ] All 17 payloads and mutations
- [ ] ordering/idempotency/tamper journal
- [ ] missing/deleted parent
- [ ] revisions/renditions/trash
- [ ] history backfill
- [ ] live updates
- [ ] cursor resume/restart catch-up match Go.

## Safety / negative cases

- [ ] No duplicate remote writes, out-of-order commit, silent unknown-op skip, or false success on cursor gaps.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

Golden fixture per op; property/fuzz; transaction interruption; replay/history fake tests; isolated canary.

## Rollback

Rebuild only on clone; preserve original cursor/journal on mismatch.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
