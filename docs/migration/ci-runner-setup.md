# Rust Migration CI — runner setup and validation

The code-change branch uses GitLab CI and **does not deploy** anything. The checked-out branch requires the existing **Oracle A1 ARM64** project runner **#59776971**, tag **oracle-arm64**, to be assigned to this GitLab project before checks will start.

In GitLab open: my_telegram_drive → Settings → CI/CD → Runners → Other available project runners → assign Oracle-A1-ARM64. If the runner is locked to Pmanager, an owner must unlock it there first. This is a GitLab project setting; a repository commit cannot assign it.

The pipeline contains:

- legacy-contracts: static Go schema/wire/API/Docker inventory
- rust-contracts: Rust fmt, clippy, all workspace unit/integration tests
- legacy-go-projection: Go 1.25.13 parity fixtures + projection regression
- legacy-web-checks: Bun 1.4.3 locked install, lint, unit tests

A pipeline in pending/created/skipped/canceled state is **not** a successful test. The exact job status and source SHA must be inspected before calling a Phase verified. Because only the Go oracle and a partial Rust wire/TDE1 inspector exist so far, no claims of full Rust backend, GPUI desktop, production Docker, or complete feature parity are justified.

To manually run the same checks once a Rust toolchain is installed:

~~~sh
python3 scripts/check-legacy-contracts.py
go test ./backend/projection -run TestRustTDX1Fixtures -count=1
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
~~~

Do not merge or deploy while Rust/compiler/test results are unknown.
