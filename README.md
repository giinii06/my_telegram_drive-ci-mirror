# TDrive

TDrive is a Telegram-backed cloud drive with a web workspace and command-line interface. The web UI is the main application; both interfaces share the same daemon, storage, and mount backend.

## Build and start

Requires Go 1.25.13 or newer.

```sh
mkdir -p build/bin
go build -trimpath -o build/bin/tdrive ./cmd/tdrive
build/bin/tdrive setup --api-id YOUR_ID --api-hash-stdin
build/bin/tdrive login +15551234567
build/bin/tdrive drives
build/bin/tdrive drive use <name-or-id>
build/bin/tdrive web --port 8080
```

Open the URL printed by `tdrive web`. By default the server listens on the Tailscale IPv4 address and uses a persisted access token. For localhost use `--listen 127.0.0.1`.

## Web workspace

Browse and search files, switch between list and icon views, upload files or folders, create folders, rename, copy, move, and delete entries. Preview images, text and PDFs, stream video and audio, and download files. Navigation supports breadcrumbs, history, keyboard shortcuts, and multi-selection.

The UI lives in `cmd/tdrive/webui/index.html` and is embedded directly into the CLI binary. After a UI change, rebuild the CLI and restart the web service. No desktop runtime or separate production frontend build is required.

For the supplied user service, install the CLI to `~/.local/bin/tdrive`, copy `scripts/tdrive-web.service` to `~/.config/systemd/user/`, and run `systemctl --user daemon-reload` followed by `systemctl --user enable --now tdrive-web.service`. The supplied service uses `--no-auth` on the Tailscale address; omit that option to use token authentication.

### Run the web service on Linux

```sh
mkdir -p ~/.local/bin ~/.config/systemd/user
install -m 755 build/bin/tdrive ~/.local/bin/tdrive
cp scripts/tdrive-web.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now tdrive-web.service
```

After rebuilding, install the updated binary under a temporary name and replace the existing executable before restarting the service:

```sh
install -m 755 build/bin/tdrive ~/.local/bin/tdrive.next
mv ~/.local/bin/tdrive.next ~/.local/bin/tdrive
systemctl --user restart tdrive-web.service
systemctl --user status tdrive-web.service
```

## Command-line interface

The CLI is available for macOS, Linux, and Windows amd64. Most commands automatically start a local daemon that keeps the Telegram connection, sync state, and unlocked vault key warm between commands.

Web and CLI commands connect to the shared daemon; a process lock prevents multiple backends from modifying the same local state.

### Install on macOS or Linux

Download and extract the CLI archive for your platform. Open a terminal in the extracted `TDrive-...-cli` directory, then run:

```bash
./install-cli.sh
```

Reload the shell if the installer updated its configuration, then set up and log in:

```bash
tdrive version
tdrive setup --api-id YOUR_ID --api-hash-stdin
tdrive login +15551234567
tdrive whoami
tdrive ls
```

### Run on Windows

Extract the `*-windows-amd64-cli.zip` release asset and run `tdrive.exe` from PowerShell:

```powershell
.\tdrive.exe setup --api-id YOUR_ID --api-hash-stdin
.\tdrive.exe login +15551234567
.\tdrive.exe whoami
```

### Common commands

```bash
tdrive drives
tdrive drive use <name|id>
tdrive ls -l
tdrive mkdir -p /Photos
tdrive put photo.jpg /Photos/
tdrive get /Photos/photo.jpg .
tdrive mkdir -p /Imports
tdrive put --extract archive.zip /Imports/
tdrive mv /old /new
tdrive rm -r /folder
tdrive sync
tdrive rebuild
tdrive unlock
tdrive mount
tdrive mount status
tdrive mount stop
```

Folder and archive imports require the destination folder to exist. Single-file uploads can create or rename the final file path.

Shared-drive commands include `drive create`, `drive link`, `drive join`, `drive requests`, `drive approve`, `drive deny`, and `drive leave`. Run `tdrive help` or `tdrive <command> --help` for the complete command reference.

### Automation and agent use

Start with the offline version and command manifest. Neither needs a daemon or Telegram connection. `tdrive commands --json` is the authoritative list of JSON-supported commands, flags, and mutation behavior.

```bash
tdrive version --json
tdrive commands --json
```

After interactive setup and login, use JSON output and explicit drive IDs for scripts:

```bash
tdrive drives --json
tdrive ls /Photos --drive-id 123456789 --json --non-interactive
tdrive put photo.jpg /Photos/photo.jpg --drive-id 123456789 --json --non-interactive
```

Use the numeric drive ID returned by `tdrive drives --json`, replacing the example ID above. Drive-scoped JSON commands require `--drive-id` and canonical absolute remote paths. They do not change the daemon's shared active drive or working directory, so concurrent scripts can target different drives safely. Human-readable output remains the default; `--json` (or `--output json`) is opt-in and rejects unsupported commands.

<details>
<summary><strong>JSON contract, exit codes, and automation limits</strong></summary>

Each successful JSON command writes one schema-versioned object to stdout (`schema_version`, `ok`, `command`, `data`), with no progress text. An error writes one object to stderr (`schema_version`, `ok: false`, `error` with `code`, `message`, `retryable`, and optional `hint`). Branch on `error.code`, not the message.

| Exit code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Other operation failure |
| `2` | Invalid or unsupported command |
| `3` | Required input or authentication |
| `4` | Not found |
| `5` | Conflict or required confirmation |
| `6` | Timeout or unavailability |

- `--non-interactive` never prompts. Vault unlock requires `--password-stdin` in JSON or non-interactive mode.
- JSON `rm` and `rebuild` require `--yes`. Non-interactive text mode also requires it for destructive drive actions and full logout. There is no dry-run option.
- JSON `put` accepts one regular local file. JSON `get` requires an explicit local file path and refuses to overwrite it unless `--yes` is given. The no-clobber check is best-effort, not atomic with other writers.
- Interactive Telegram login is still required, and setup is text-only. `setup --api-id ID --api-hash-stdin` avoids putting the API hash in shell history or process arguments.
- `--timeout 30s` bounds a daemon RPC after startup, not daemon startup itself.
- `cat` emits raw file bytes and does not support JSON. It verifies downloads in a private temporary directory before writing them to stdout; a crash can leave plaintext there.

</details>

## Storage and encryption

Personal drives use private Telegram channels; shared drives use Telegram groups. File contents are stored as document messages and large files are split into parts. Small metadata messages describe folders and operations. SQLite holds a local projection that can be synchronized or rebuilt from Telegram history. Editing or deleting drive metadata messages in Telegram can break that history.

Personal-drive uploads can use client-side XChaCha20-Poly1305 encryption. File and folder names, sizes, and control metadata remain visible. Shared drives do not use this encryption. The vault key stays in daemon memory while unlocked; `tdrive vault lock` forgets it. There is no password reset that can recover encrypted contents.

Configuration and local state use the operating system's user configuration directory under `TDrive`; temporary data and media caches use its user cache directory. Logging out, clearing the local projection, and deleting remote files are separate operations.

## Mounts

`tdrive mount` starts a private localhost WebDAV endpoint and attaches the selected drive to the operating system. Use `tdrive mount status` and `tdrive mount stop` to inspect or detach it. macOS and Linux mount helpers and Windows support remain available through the CLI. Shared and encrypted drives mount read-only; eligible personal plaintext drives support writes.

Media conversion and video-poster extraction use optional system tools such as FFmpeg and mpv when available.

## Development checks

```sh
go test ./cmd/tdrive ./cmd/tdrived ./backend/...
go vet ./cmd/tdrive ./cmd/tdrived ./backend/...
npm ci --prefix cmd/tdrive/webui
npm --prefix cmd/tdrive/webui run lint
bash scripts/test-web-ui.sh
cd cmd/tdrive/webui
npx playwright install chromium
npm run test:browser
```

Web styles use semantic CSS variables for surfaces, foregrounds, borders, selection, and focus states. Shadcn Lint enforces semantic utility colors and rejects arbitrary utility values in the HTML. Browser tests verify desktop and narrow-screen layouts. See [build instructions](build/README.md) for release packaging.

## License

See [LICENSE](LICENSE).
