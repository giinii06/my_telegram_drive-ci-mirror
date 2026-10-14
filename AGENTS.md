# TDrive agent notes

## Runtime: Docker first

The live instance runs in Docker Compose (`docker-compose.yml`, service `tdrive`, image `tdrive:latest`), not via host systemd units or `~/.local/bin`. There is no host install: no `tdrive` on PATH, no `~/TDrive` symlink, no user services.

- `docker compose up -d [--build]` / `ps` / `logs -f tdrive` / `down`
- Run the CLI inside the container: `docker compose exec tdrive tdrive <args>` (daemon status, drives, vault, mount, ls, ...)
- State lives in volume `tdrive_tdrive-data` mounted at `/data` (`HOME=/data`): config in `/data/.config/TDrive`, cache in `/data/.cache/TDrive`.
- Web serves on `0.0.0.0:8080` with `--no-auth`: tailnet peers via the sidecar, host via `127.0.0.1:8080` publishing on the tailscale service.
- Networking: `tailscale` sidecar (node `tdrive`, official `tailscale/tailscale:stable` image, state in `tdrive-ts-state` volume); `tdrive` uses `network_mode: service:tailscale`. Auth key lives in `.env` (`TS_AUTHKEY`, gitignored, `tskey-auth-` key). Recreating the tailscale container orphans tdrive's netns → follow with `docker compose up -d --force-recreate tdrive`.
- Entrypoint (`docker/entrypoint.sh`): dbus → `daemon start --background` → wait for readiness → vault auto-unlock from `/data/.config/TDrive/vault.pass` when present → wait for a selected drive (`tdrive drives` shows `*`) → `tdrive web`.
- Container flags (set in `docker-compose.yml` env): `TDRIVE_NO_OS_MOUNT=1`, `TDRIVE_CONTAINER=1`.

## Container code paths (do not regress)

- `headlessContainerMode()` in `backend/mountcontroller/controller.go`: skips the OS attach and uses a `container`-kind pseudo-attachment; `runStop` and endpoint recovery skip `Detach` for it. Gate on `Attachment.Kind() != "container"`, not on env vars at the call sites.
- `resolveMountDAVEndpointFromDaemon()` in `cmd/tdrive/web.go`: fallback when the `~/TDrive` symlink is missing; parses the daemon `MountStatus.Location` URL. Keep the symlink path first (bare metal still uses it).

## Layout

- `cmd/tdrive` — CLI + embedded web UI (`webui/index.html` via `web_ui.go`) + daemon auto-start (`daemon_process.go`).
- `cmd/tdrived` — daemon binary entry.
- `backend/daemon` — daemon server, IPC protocol (`protocol.go`), mount orchestration (`mount.go`).
- `backend/mountcontroller`, `backend/mountos`, `backend/mountdav`, `backend/mountfs` — mount lifecycle, OS connectors, loopback WebDAV endpoint, filesystem.
- `backend/core`, `backend/services/...`, `backend/projection`, `backend/tgclient` — engine, Telegram client, local projection.
- `scripts/` — release packaging, backup/open helpers, `tdrive-*.service` units (bare-metal alternative only).
- `Dockerfile` (multi-stage; runtime is debian-slim + tini + dbus + gvfs-backends), `docker-compose.yml`, `docker/entrypoint.sh`.

## Dev loop

```sh
go build -trimpath -o build/bin/tdrive ./cmd/tdrive
go test ./cmd/tdrive ./cmd/tdrived ./backend/...
go vet ./cmd/tdrive ./cmd/tdrived ./backend/...
npm ci --prefix cmd/tdrive/webui && npm --prefix cmd/tdrive/webui run lint
docker compose up -d --build   # any repo change invalidates the builder COPY layer; a full Go rebuild takes minutes
```

## Safety rules

- Never print or log secrets: API hash (`imp_config.json`), `vault.pass`, web tokens, mount capability URLs/prefixes, phone numbers, login codes/2FA. `mountcontroller.Status` and daemon `MountResponse` must stay capability-free.
- `backend.lock` is a runtime PID lock: never copy it into volumes or images; never ship host state into the image.
- `.env` (Tailscale auth key) is gitignored: never commit it, never print it.
- Destructive CLI ops (`rm`, `rebuild`, full `logout`) require confirmation flags; keep that behavior in both text and JSON modes.
