# Phase 02 — Rust 실행 기반·설정·SQLite v16 저장소

**Kind:** foundation  
**Initial status:** not_started  
**Goal:** Rust server can safely read/write existing TDrive data under one process/SQLite writer; consumers are Phases 03–10.

## Repository surface

- `Cargo.toml`
- `crates/tdrive-types`
- `crates/tdrive-config`
- `crates/tdrive-storage`
- `crates/tdrive-projection`
- `apps/tdrive-server`
- `apps/tdrive-cli`
- `tests/compatibility/storage/**`

## Acceptance criteria

- [ ] Full schema v16 migration, transactions, WAL, crash recovery, config/session permissions, process lock, Go/Rust digest parity, snapshot/restore clone.

## Safety / negative cases

- [ ] Never mutate the only DB copy or permit Go/Rust concurrent writers. Corrupt/symlink/permission errors fail closed.

## Test-first steps and evidence

1. Add a behavior/contract test first; record the exact test-only SHA and command.
2. Observe RED for the intended assertion failure. If no executor or job logs are available, record RED as unverified.
3. Implement the minimum behavior; observe GREEN and run related regression tests.
4. Run the applicable repository checks on the candidate SHA: `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 test --workspace --locked`, `bun install --cwd cmd/tdrive/webui --frozen-lockfile`, `bun run --cwd cmd/tdrive/webui lint`, `bun run --cwd cmd/tdrive/webui test:unit`, and `bun run --cwd cmd/tdrive/webui test:browser` when relevant.
5. Inspect effective CI provider, exact checkout SHA, pipeline/run ID, jobs, commands, logs and E2E separately. Dispatch/provider callback alone is not job proof.
6. Record separate requirements and security/quality reviews. If only self-review is available, label it self-review.
7. Commit phase evidence and append-only phase-state checkpoint before advancing.

## CI / verification

cargo +1.92.0 fmt/clippy/test --workspace --locked; Go-generated v16 fixture tests; lock concurrency; WAL restore/crash drills.

## Rollback

Migrate only a clone; preserve original DB and verified backup.

## Exit gate

- [ ] Acceptance and negative cases have exact-SHA evidence
- [ ] No unresolved Critical/Important review finding
- [ ] Phase-state history and review report are committed
- [ ] Downstream consumer/user flow remains identified
