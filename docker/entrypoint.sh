#!/bin/sh
set -e

export HOME="${HOME:-/data}"
export TDRIVE_NO_OS_MOUNT=1
export TDRIVE_CONTAINER=1

mkdir -p "$HOME/.config" "$HOME/.cache"

if command -v service >/dev/null 2>&1; then
    service dbus start >/dev/null 2>&1 || true
fi
if ! pgrep -x dbus-daemon >/dev/null 2>&1 && command -v dbus-daemon >/dev/null 2>&1; then
    dbus-daemon --system >/dev/null 2>&1 || true
fi

tdrive daemon start --background >/dev/null 2>&1 || true

for _ in $(seq 1 50); do
    if tdrive daemon status >/dev/null 2>&1; then
        break
    fi
    sleep 0.2
done

# Auto-unlock vault when a password file was migrated (host vault.service equivalent).
if [ -f "$HOME/.config/TDrive/vault.pass" ]; then
    tdrive unlock --password-stdin < "$HOME/.config/TDrive/vault.pass" >/dev/null 2>&1 || true
fi

# Wait until a drive has been selected.
while ! tdrive drives 2>/dev/null | grep -q '^[*]'; do
    echo "tdrive-entrypoint: waiting for a selected drive (run: docker compose exec tdrive tdrive drive use <name|id>)" >&2
    sleep 5
done

PORT="${PORT:-8080}"
tdrive web --listen 0.0.0.0 --port "$PORT" --no-auth &
WEB_PID=$!

cleanup() {
    kill "$WEB_PID" >/dev/null 2>&1 || true
    wait "$WEB_PID" >/dev/null 2>&1 || true
    tdrive daemon stop >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

wait "$WEB_PID"
