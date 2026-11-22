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

## Bounded Telegram → native SQLite inbox

These commands use a **separate native Rust inbox**, never the Go v16 database, full sync projector, or production vault. They only read a bounded recent Telegram history page and accept valid TDX1 operations. Missing older history, deleted messages, cursor gaps and live update subscriptions are **not handled**.

```sh
mkdir -p "$HOME/.local/share/tdrive-mtproto"
chmod 700 "$HOME/.local/share/tdrive-mtproto"
export TDRIVE_INBOX_PATH="$HOME/.local/share/tdrive-mtproto/tdrive-inbox.sqlite"
# This command is OFFLINE: it does not require TG_API_ID or TG_API_HASH.
apps/tdrive-mtproto/target/release/tdrive-mtproto inbox-stats
# On a new account DB, first run login with developer API credentials,
# then ingest a bounded sample of already-visible Telegram messages:
apps/tdrive-mtproto/target/release/tdrive-mtproto ingest '@publicchannel' 100
```

On the first run, `inbox-stats` refuses to create a missing database; run `ingest` to create it on a disposable account. Repeated `ingest` of the same Telegram message keeps the first header and reports edited headers separately. **This is diagnostic metadata, not remotely encrypted file backup or sync parity.**

### Optional Compose integration (no change to the Go stack)

The new `docker-compose.rust.yml` does not replace `docker-compose.yml`. Create a private host directory and set the UID/GID:

```sh
export TDRIVE_RUST_UID="$(id -u)" TDRIVE_RUST_GID="$(id -g)"
export TDRIVE_RUST_DATA="$HOME/.local/share/tdrive-mtproto"
mkdir -p "$TDRIVE_RUST_DATA"
chmod 700 "$TDRIVE_RUST_DATA"
# Works without Telegram API credentials *if the inbox DB already exists*:
docker compose -f docker-compose.rust.yml run --rm rust-mtproto inbox-stats
# Network/login/ingest commands require these user-supplied credentials.
export TG_API_ID='YOUR_API_ID' TG_API_HASH='YOUR_32_CHARACTER_API_HASH'
docker compose -f docker-compose.rust.yml run --rm rust-mtproto login
docker compose -f docker-compose.rust.yml run --rm rust-mtproto ingest '@publicchannel' 100
```

The container is non-root, read-only apart from the explicitly mounted private folder, has no published ports and runs **one command per invocation**, not a persistent headless service. Use only a disposable test account pending Cargo/Docker and live integration E2E validation.

## Safety and limitations

- Upload requires an explicit `UPLOAD` confirmation and never retries an ambiguous Telegram send automatically. Check message history before manual retry.
- Download stages to a private `0600` same-directory file, then publishes with a no-clobber hard link; errors remove the staged file. The result destination must not exist.
- `history` retrieves at most 100 messages per call and dialogs are capped. Telegram flood waits or network errors still require caller recovery.
- Credentials are read only from environment, not argv; the session path must be absolute, with a private parent directory and no final symlink. Ancestor-symlink race, session DB file-lock lifetime, secure password zeroization and cross-device behavior still require review.
- No Go SQLite v16 projection mutation or atomic 17-operation replay, idempotent MTProto write scheduler, encryption vault migration, HLS/WebDAV, GPUI, or production Docker daemon is integrated.
- Cargo `fmt`, Clippy, tests, release binary, Docker image and real login/upload/download have **not been executed** in this chat environment (no Rust compiler, private checkout or test account). The GitHub workflow is a source definition and needs real job evidence.
- Preserve original Go code and operating data until the Rust implementation achieves full verified parity, migration, recovery and rollback. Do not deploy this diagnostic to production.

See [C.1 migration phase](../../docs/plans/rust-only-cutover/phase-c1-user-mtproto.md) and [full Go-free cutover plan](../../docs/plans/rust-only-cutover/README.md).
