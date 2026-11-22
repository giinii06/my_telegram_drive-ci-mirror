# Native Rust Telegram user client — experimental C.1

This is a separate **actual MTProto user-account** executable, using Grammers, not the Bot API and not a Go subprocess. It has **not** been compiled, run against Telegram, or integrated into the live v16 projection/vault yet. The existing Go server stays authoritative.

The program is a nested Cargo workspace (`apps/tdrive-mtproto/Cargo.toml`). Its new Grammers dependency graph is not in the root `Cargo.lock`: a Cargo executor must generate and commit a reproducible **nested lockfile** after validating compatible versions. No checks should claim `--locked` success yet.

## Local macOS and Linux

Create a private session directory, configure the Telegram developer credentials from https://my.telegram.org, then build and run:

```sh
mkdir -p "$HOME/.local/share/tdrive-mtproto"
chmod 700 "$HOME/.local/share/tdrive-mtproto"
export TG_API_ID='YOUR_API_ID'
export TG_API_HASH='YOUR_32_CHARACTER_API_HASH'
export TDRIVE_SESSION_PATH="$HOME/.local/share/tdrive-mtproto/session.sqlite"
cargo +1.92.0 build --release --manifest-path apps/tdrive-mtproto/Cargo.toml
BIN=apps/tdrive-mtproto/target/release/tdrive-mtproto
"$BIN" --help
"$BIN" login
"$BIN" status
"$BIN" dialogs
"$BIN" history '@publicchannel' 20
# Confirm carefully: upload sends a REAL Telegram document message.
# "$BIN" upload '@publicchannel' '/absolute/path/to/file.bin'
# "$BIN" download '@publicchannel' 123 '/absolute/new-file.bin'
```

`login` prompts for the phone, Telegram code and optional hidden 2FA password; it does not accept them on the command line. `dialogs` prints peer identifiers for selecting private chats; raw channel IDs cannot bypass Telegram access hashes or authentication. The local `0700` session directory holds an account session DB with authorization material. Do not share it, commit it, mount it publicly or use a real account for the current unverified E2E.

## Headless Docker image

This image runs one-off manual CLI commands. It has **no exposed ports, no public API, no secret baked into layers and no background daemon**.

```sh
docker build -f Dockerfile.mtproto -t tdrive-mtproto:local .
mkdir -p "$HOME/.local/share/tdrive-mtproto"
chmod 700 "$HOME/.local/share/tdrive-mtproto"
export TG_API_ID='YOUR_API_ID'
export TG_API_HASH='YOUR_32_CHARACTER_API_HASH'
docker run --rm -it \
  --user "$(id -u):$(id -g)" \
  -e TG_API_ID -e TG_API_HASH \
  -e TDRIVE_SESSION_PATH=/private/session.sqlite \
  -v "$HOME/.local/share/tdrive-mtproto:/private:rw" \
  tdrive-mtproto:local login
```

For subsequent commands, use the same private volume and environment, changing `login` to `status`, `dialogs`, or `history`. Run all network tests first using an isolated disposable account.

## Safety and limitations

- Upload requires an explicit `UPLOAD` confirmation and never retries an ambiguous Telegram send automatically. Check message history before manual retry.
- Download stages to a private `0600` same-directory file, then publishes with a no-clobber hard link; errors remove the staged file. The result destination must not exist.
- `history` retrieves at most 100 messages per call and dialogs are capped. Telegram flood waits or network errors still require caller recovery.
- Credentials are read only from environment, not argv; the session path must be absolute, with a private parent directory and no final symlink. Ancestor-symlink race, session DB file-lock lifetime, secure password zeroization and cross-device behavior still require review.
- No Go SQLite v16 projection mutation or atomic 17-operation replay, idempotent MTProto write scheduler, encryption vault migration, HLS/WebDAV, GPUI, or production Docker daemon is integrated.
- Cargo `fmt`, Clippy, tests, release binary, Docker image and real login/upload/download have **not been executed** in this chat environment (no Rust compiler, private checkout or test account). The GitHub workflow is a source definition and needs real job evidence.
- Preserve original Go code and operating data until the Rust implementation achieves full verified parity, migration, recovery and rollback. Do not deploy this diagnostic to production.

See [C.1 migration phase](../../docs/plans/rust-only-cutover/phase-c1-user-mtproto.md) and [full Go-free cutover plan](../../docs/plans/rust-only-cutover/README.md).
