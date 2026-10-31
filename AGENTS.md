# TDrive agent notes

## Runtime: Docker only

TDrive runs only in Docker Compose (`docker-compose.yml`, service `tdrive`, image `tdrive:latest`), not via host systemd units or standalone binaries. `install.sh` is the supported bootstrap: it clones the stack to `~/.tdrive`, writes `.env`, runs Compose (pulling the published image on amd64, otherwise building locally), and installs a thin `~/.local/bin/tdrive` wrapper that only delegates to `docker compose exec`.

- `docker compose up -d [--build]` / `ps` / `logs -f tdrive` / `down`
- Run the CLI inside the container: `docker compose exec tdrive tdrive <args>` (daemon status, drives, vault, mount, ls, ...)
- State lives in volume `tdrive_tdrive-data` mounted at `/data` (`HOME=/data`): config in `/data/.config/TDrive`, cache in `/data/.cache/TDrive`.
- Web serves on `0.0.0.0:8080` with `--no-auth`, reachable over the tailnet only (no ports published).
- Networking: `tailscale` sidecar (node `tdrive`, official `tailscale/tailscale:stable` image, state in the Compose `tdrive-ts-state` volume); `tdrive` uses `network_mode: service:tailscale`. Auth key lives in `.env` (`TS_AUTHKEY`, gitignored, `tskey-auth-` key). Recreating the tailscale container orphans tdrive's netns → follow with `docker compose up -d --force-recreate tdrive`.
- Entrypoint (`docker/entrypoint.sh`): `daemon start --background` → wait for readiness → vault auto-unlock from `/data/.config/TDrive/vault.pass` when present → `tdrive web` immediately. The web server starts before a drive is selected so browser onboarding can run.
- Container flags (set in `Dockerfile` and `docker-compose.yml` env): `TDRIVE_NO_OS_MOUNT=1`, `TDRIVE_CONTAINER=1`.

## Bootstrap installer

- `install.sh` targets Linux servers with `/dev/net/tun`, Docker, and Compose v2. It must not install a host daemon, systemd unit, or standalone binary. It ends at the web URL; Telegram onboarding is browser-only.
- The `~/.local/bin/tdrive` wrapper is part of the bootstrap. It is a `docker compose exec` shim, not a host CLI; keep the `TDRIVE_CONTAINER=1` gate in `main()` intact.
- Prompts must read from `/dev/tty` because `curl | bash` consumes stdin for the script itself.
- On amd64 the installer pulls `ghcr.io/giinii06/my_telegram_drive:latest`; other architectures and pull failures fall back to `docker compose up -d --build`.
- The GHCR package is private until made public in the repository package settings. Keep the installer working without registry credentials.
- Never echo `TS_AUTHKEY`, Telegram API credentials, login codes, or 2FA values.

## Web onboarding

- `cmd/tdrive/web_onboarding.go` exposes `/api/onboarding/{status,setup,login,code,password,drives,drive}`. Setup is allowed only while logged out; code/password only during an active flow; drive selection only while logged in.
- The login flow runs `LoginContext` in a goroutine and polls `auth.status`; never block an HTTP handler on the streaming login call.
- `LoginContext` closes the daemon socket on cancel. The web flow times out after 10 minutes, and `POST /api/onboarding/cancel` resets it so a closed browser cannot leave the daemon's login flow stuck.
- Onboarding POSTs require `Content-Type: application/json`. Never log or echo API hashes, phone numbers, login codes, or 2FA passwords.
- `runWeb` tolerates a missing mount when no drive is active. Start the mount after drive selection (`handleOnboardingDrive`) or at startup when a drive already exists.

## Container code paths (do not regress)

- `headlessContainerMode()` in `backend/mountcontroller/controller.go`: skips the OS attach and uses a `container`-kind pseudo-attachment; `runStop` and endpoint recovery skip `Detach` for it. Gate on `Attachment.Kind() != "container"`, not on env vars at the call sites.
- `resolveMountDAVEndpointFromDaemon()` in `cmd/tdrive/web.go`: fallback when the `~/TDrive` symlink is missing; parses the daemon `MountStatus.Location` URL. Keep the web-to-WebDAV connection working when there is no host mount or symlink.

## Layout

- `cmd/tdrive` — CLI + embedded web UI (`webui/index.html` via `web_ui.go`) + daemon auto-start (`daemon_process.go`).
- `backend/daemon` — daemon server, IPC protocol (`protocol.go`), mount orchestration (`mount.go`).
- `backend/mountcontroller`, `backend/mountos`, `backend/mountdav`, `backend/mountfs` — mount lifecycle, OS connectors, loopback WebDAV endpoint, filesystem.
- `backend/core`, `backend/services/...`, `backend/projection`, `backend/tgclient` — engine, Telegram client, local projection.
- `cmd/tdrive/webui/checks/` — Bun folder-upload checks. `tests/` contains Playwright browser checks, including `onboarding.spec.mjs`.
- `cmd/tdrive/web_onboarding.go` — browser onboarding routes and login flow state.
- `install.sh` — Linux bootstrap for the Compose stack and the `tdrive` wrapper.
- `Dockerfile` (multi-stage; runtime is debian-slim + ca-certificates + tini), `docker-compose.yml`, `docker/entrypoint.sh`.

## Dev loop

```sh
bash -n install.sh
TDRIVE_DRY_RUN=1 bash install.sh
docker compose build tdrive
go test ./cmd/tdrive ./backend/...
go vet ./cmd/tdrive ./backend/...
bun install --cwd cmd/tdrive/webui --frozen-lockfile
bun run --cwd cmd/tdrive/webui lint
bun run --cwd cmd/tdrive/webui browser:install
bun run --cwd cmd/tdrive/webui test
docker compose up -d --build   # any repo change invalidates the builder COPY layer; a full Go rebuild takes minutes
```

## Safety rules

- Never print or log secrets: API hash (`imp_config.json`), `vault.pass`, web tokens, mount capability URLs/prefixes, phone numbers, login codes/2FA. `mountcontroller.Status` and daemon `MountResponse` must stay capability-free.
- `backend.lock` is a runtime PID lock: never copy it into volumes or images; never ship host state into the image.
- `.env` (Tailscale auth key) is gitignored: never commit it, never print it.
- Destructive CLI ops (`rm`, `rebuild`, full `logout`) require confirmation flags; keep that behavior in both text and JSON modes.

## Supported deployment surface

- Keep installation and deployment Docker-only. `install.sh` may bootstrap Compose and install the wrapper; do not reintroduce host daemons, systemd units, standalone daemon binaries, or OS-specific CLI release packages.
- CI publishes `ghcr.io/<repo>` on pushes to `main` and `v*` tags. Only the `publish` job gets `packages: write`.
- The CLI is an in-container administration tool. `main()` rejects execution without `TDRIVE_CONTAINER=1`; tests may call `run()` directly. The flag identifies the supported runtime, not a security sandbox.
- The `~/.local/bin/tdrive` wrapper must stay a pass-through to `docker compose exec`; it must not implement CLI behavior or call the daemon socket directly.
- Internal mount controller and `mountos` types remain shared implementation details for the container WebDAV lifecycle. They are not a supported host deployment path.
- CI builds the Docker image and runs offline version/manifest checks without secrets or live state.

## JavaScript tooling

- Use Bun 1.4.3 for dependency installation, scripts, ESLint, and Playwright. Keep `bun.lock` as the only JavaScript lockfile.
- `cmd/tdrive/webui/bunfig.toml` forces package scripts and their CLI shebangs to run with Bun.
- Install browsers with `bun run --cwd cmd/tdrive/webui browser:install`; append `--with-deps` on Linux when system dependencies are needed.
