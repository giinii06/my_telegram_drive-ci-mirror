#!/usr/bin/env bash
# Private-source verification only. Never authenticate to Telegram or publish Git history.
set -Eeuo pipefail
set +x

fail() { printf 'tdrive-local-check: %s\n' "$*" >&2; exit 1; }
step() { printf '\n[tdrive-local-check] %s\n' "$*"; }

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd -- "$root"
[[ "$(git rev-parse --show-toplevel 2>/dev/null)" == "$root" ]] || fail 'run from a Git checkout root'
sha="$(git rev-parse --verify HEAD)"
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || fail 'invalid checkout SHA'
if [[ -n "${TDRIVE_EXPECTED_SHA:-}" && "$sha" != "$TDRIVE_EXPECTED_SHA" ]]; then
  fail 'checkout SHA mismatch (refusing to test a different commit)'
fi
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || fail 'checkout is dirty; review/commit changes before testing'
step "private checkout SHA: $sha"

[[ -f Cargo.toml && -f apps/tdrive-mtproto/Cargo.toml ]] || fail 'missing Rust workspace manifest'
[[ -s Cargo.lock ]] || fail 'root Cargo.lock missing; resolve and review it on a trusted private checkout before retrying'
if [[ ! -s apps/tdrive-mtproto/Cargo.lock ]]; then
  fail 'nested Cargo.lock missing; on a trusted machine generate it with: cargo +1.92.0 generate-lockfile --manifest-path apps/tdrive-mtproto/Cargo.toml; audit and commit before retrying'
fi
command -v cargo >/dev/null 2>&1 || fail 'cargo not installed (Rust 1.92 required)'
command -v rustc >/dev/null 2>&1 || fail 'rustc not installed (Rust 1.92 required)'
cargo +1.92.0 --version >/dev/null || fail 'Rust 1.92 toolchain not installed'
rustc +1.92.0 --version >/dev/null || fail 'Rust 1.92 compiler not installed'

step 'fmt root workspace (source only)'
cargo +1.92.0 fmt --all --check
step 'clippy root Rust workspace with locked deps'
cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings
step 'test root Rust workspace with locked deps'
cargo +1.92.0 test --workspace --all-targets --locked
step 'build root Rust workspace release with locked deps'
cargo +1.92.0 build --workspace --release --locked
step 'fmt standalone Telegram workspace'
cargo +1.92.0 fmt --manifest-path apps/tdrive-mtproto/Cargo.toml --check
step 'clippy standalone Telegram workspace with locked deps'
cargo +1.92.0 clippy --manifest-path apps/tdrive-mtproto/Cargo.toml --all-targets --locked -- -D warnings
step 'test standalone Telegram workspace with locked deps'
cargo +1.92.0 test --manifest-path apps/tdrive-mtproto/Cargo.toml --locked
step 'build standalone Telegram workspace release with locked deps'
cargo +1.92.0 build --manifest-path apps/tdrive-mtproto/Cargo.toml --release --locked
step 'run safe offline help (never Telegram login/upload/watch)'
apps/tdrive-mtproto/target/release/tdrive-mtproto --help

if [[ "${TDRIVE_CHECK_DOCKER:-0}" == 1 ]]; then
  command -v docker >/dev/null 2>&1 || fail 'Docker requested but not installed'
  step 'validate private non-root watch Compose with dummy values'
  TG_API_ID=12345 TG_API_HASH=00000000000000000000000000000000 \
    TDRIVE_RUST_UID=10001 TDRIVE_RUST_GID=10001 \
    TDRIVE_RUST_DATA=/tmp/tdrive-ci-private TDRIVE_RUST_PEER=@testaccount \
    docker compose -f docker-compose.rust-watch.yml config --quiet
  step 'build local image; do not publish'
  docker build --file Dockerfile.mtproto --tag tdrive-mtproto:local-check .
  step 'offline container help (network disabled, no credentials or session)'
  docker run --rm --network none tdrive-mtproto:local-check --help
fi
step "PASS: local Rust checks finished at SHA $sha (no live Telegram E2E performed)"
