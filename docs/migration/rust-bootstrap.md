# Rust workspace bootstrap (Phase 03, initial increment)

A Cargo workspace now exists alongside the original Go application. Its first deliverables are **small offline contract tools**, not a running Telegram drive or a replacement production server. No Go daemon, web UI, Dockerfile, or installation script has been replaced.

- crates/tdrive-types: checked positive Telegram identifiers, 64-bit-safe decimal API IDs.
- crates/tdrive-projection: reserved TDX1 compatibility module, initially read-only.
- apps/tdrive-inspect: separate executable for verifying the wire protocol version without accessing local data or credentials.
- .gitlab-ci.yml: source inventory check, Rust fmt/clippy/tests, legacy Go projection tests.

Target toolchain: Rust 1.92.0; language feature minimum initially 1.85 (edition 2024). The desktop GPUI crate and Telegram MTProto client are **not** included until exact upstream versions and host target builds are verified.

~~~sh
python3 scripts/check-legacy-contracts.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run --locked -p tdrive-inspect -- --wire-version
~~~

The connected GitLab project currently reports shared runners disabled. CI must be observed on the **matching commit SHA**; config creation alone is not execution. Until Rust CI is green, this workspace is not release-ready.

Next real milestones: TDX1 offline parser + Go golden vectors, SQLite read-only migration inventory, and isolated MTProto engine spike. A complete Rust/GPUI/Docker replacement is not claimed by this bootstrap.
