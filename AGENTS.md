# TDrive agent notes

## Runtime: Docker only

TDrive runs only in Docker Compose (`docker-compose.yml`, service `tdrive`, image `tdrive:latest`), not via host systemd units or `~/.local/bin`. There is no host install: no `tdrive` on PATH, no `~/TDrive` symlink, no user services.

- `docker compose up -d [--build]` / `ps` / `logs -f tdrive` / `down`
- Run the CLI inside the container: `docker compose exec tdrive tdrive <args>` (daemon status, drives, vault, mount, ls, ...)
- State lives in volume `tdrive_tdrive-data` mounted at `/data` (`HOME=/data`): config in `/data/.config/TDrive`, cache in `/data/.cache/TDrive`.
- Web serves on `0.0.0.0:8080` with `--no-auth`, reachable over the tailnet only (no ports published).
- Networking: `tailscale` sidecar (node `tdrive`, official `tailscale/tailscale:stable` image, state in the Compose `tdrive-ts-state` volume); `tdrive` uses `network_mode: service:tailscale`. Auth key lives in `.env` (`TS_AUTHKEY`, gitignored, `tskey-auth-` key). Recreating the tailscale container orphans tdrive's netns → follow with `docker compose up -d --force-recreate tdrive`.
- Entrypoint (`docker/entrypoint.sh`): `daemon start --background` → wait for readiness → vault auto-unlock from `/data/.config/TDrive/vault.pass` when present → wait for a selected drive (`tdrive drives` shows `*`) → `tdrive web`.
- Container flags (set in `Dockerfile` and `docker-compose.yml` env): `TDRIVE_NO_OS_MOUNT=1`, `TDRIVE_CONTAINER=1`.

## Container code paths (do not regress)

- `headlessContainerMode()` in `backend/mountcontroller/controller.go`: skips the OS attach and uses a `container`-kind pseudo-attachment; `runStop` and endpoint recovery skip `Detach` for it. Gate on `Attachment.Kind() != "container"`, not on env vars at the call sites.
- `resolveMountDAVEndpointFromDaemon()` in `cmd/tdrive/web.go`: fallback when the `~/TDrive` symlink is missing; parses the daemon `MountStatus.Location` URL. Keep the web-to-WebDAV connection working when there is no host mount or symlink.

## Layout

- `cmd/tdrive` — CLI + embedded web UI (`webui/index.html` via `web_ui.go`) + daemon auto-start (`daemon_process.go`).
- `backend/daemon` — daemon server, IPC protocol (`protocol.go`), mount orchestration (`mount.go`).
- `backend/mountcontroller`, `backend/mountos`, `backend/mountdav`, `backend/mountfs` — mount lifecycle, OS connectors, loopback WebDAV endpoint, filesystem.
- `backend/core`, `backend/services/...`, `backend/projection`, `backend/tgclient` — engine, Telegram client, local projection.
- `cmd/tdrive/webui/checks/` — Bun folder-upload checks. `tests/` contains Playwright browser checks.
- `Dockerfile` (multi-stage; runtime is debian-slim + ca-certificates + tini), `docker-compose.yml`, `docker/entrypoint.sh`.

## Dev loop

```sh
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

- Keep installation and deployment Docker-only. Do not reintroduce host installers, systemd units, standalone daemon binaries, or OS-specific CLI release packages.
- The CLI is an in-container administration tool. `main()` rejects execution without `TDRIVE_CONTAINER=1`; tests may call `run()` directly. The flag identifies the supported runtime, not a security sandbox.
- Internal mount controller and `mountos` types remain shared implementation details for the container WebDAV lifecycle. They are not a supported host deployment path.
- CI builds the Docker image and runs offline version/manifest checks without secrets or live state.

## JavaScript tooling

- Use Bun 1.4.3 for dependency installation, scripts, ESLint, and Playwright. Keep `bun.lock` as the only JavaScript lockfile.
- `cmd/tdrive/webui/bunfig.toml` forces package scripts and their CLI shebangs to run with Bun.
- Install browsers with `bun run --cwd cmd/tdrive/webui browser:install`; append `--with-deps` on Linux when system dependencies are needed.
