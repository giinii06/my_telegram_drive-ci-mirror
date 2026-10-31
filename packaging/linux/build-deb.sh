#!/usr/bin/env bash
# Build the TDrive .deb for linux/amd64 on any host.
#
# dpkg-deb is used when it exists. Elsewhere (macOS, for example) the ar
# archive is assembled by hand, which is the same layout dpkg-deb writes.
# Set TDRIVE_DEB_NO_DPKG=1 to exercise that fallback on a host with dpkg-deb.
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PACKAGE=tdrive
ARCH=amd64

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

usage() {
  printf 'usage: packaging/linux/build-deb.sh [--version X]\n'
}

VERSION=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || die "--version requires a value"
      VERSION=$2
      shift 2
      ;;
    --version=*)
      VERSION=${1#*=}
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1 (usage: $0 [--version X])"
      ;;
  esac
done

if [ -z "$VERSION" ]; then
  VERSION=$(git -C "$ROOT" describe --tags --always --dirty 2>/dev/null || printf '0.0.0-dev')
fi
VERSION=${VERSION#v}
[ -n "$VERSION" ] || die "empty version"
COMMIT=$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || printf 'unknown')

command -v go >/dev/null 2>&1 || die "go is required"
command -v tar >/dev/null 2>&1 || die "tar is required"

OUT_DIR="$ROOT/dist"
OUT="$OUT_DIR/${PACKAGE}_${VERSION}_${ARCH}.deb"
mkdir -p "$OUT_DIR"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
PKG="$WORK/pkg"
mkdir -p "$PKG/DEBIAN" "$PKG/usr/bin" "$PKG/lib/systemd/system" "$PKG/etc/default"

printf '==> building %s %s (%s)\n' "$PACKAGE" "$VERSION" "$COMMIT"
(
  cd "$ROOT"
  CGO_ENABLED=0 GOOS=linux GOARCH=$ARCH go build -trimpath \
    -ldflags "-s -w -X main.buildVersion=$VERSION -X main.buildCommit=$COMMIT" \
    -o "$PKG/usr/bin/tdrive" ./cmd/tdrive
)

install -m 0644 "$SCRIPT_DIR/tdrive.service" "$PKG/lib/systemd/system/tdrive.service"
install -m 0644 "$SCRIPT_DIR/tdrive.default" "$PKG/etc/default/tdrive"

cat > "$PKG/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e

if ! getent passwd tdrive >/dev/null 2>&1; then
  useradd --system --home /var/lib/tdrive --shell /usr/sbin/nologin tdrive
fi
install -d -o tdrive -g tdrive -m 0700 /var/lib/tdrive

if [ -d /run/systemd/system ]; then
  systemctl daemon-reload >/dev/null 2>&1 || true
  systemctl enable tdrive.service >/dev/null 2>&1 || true
  systemctl try-restart tdrive.service >/dev/null 2>&1 || true
fi

cat <<'MESSAGE'

TDrive is installed. The web server runs as the "tdrive" system user.
Sign in to Telegram with:

  sudo -u tdrive tdrive setup
  sudo -u tdrive tdrive login
  sudo -u tdrive tdrive drives
  sudo -u tdrive tdrive drive use <name-or-id>

Then read the URL, including its access token, from:

  sudo journalctl -u tdrive | grep 'TDrive web:'

MESSAGE
exit 0
POSTINST

cat > "$PKG/DEBIAN/prerm" <<'PRERM'
#!/bin/sh
set -e

if [ -d /run/systemd/system ]; then
  if [ "$1" = "remove" ] || [ "$1" = "deconfigure" ]; then
    systemctl stop tdrive.service >/dev/null 2>&1 || true
    systemctl disable tdrive.service >/dev/null 2>&1 || true
  fi
fi
exit 0
PRERM

cat > "$PKG/DEBIAN/postrm" <<'POSTRM'
#!/bin/sh
set -e

if [ -d /run/systemd/system ]; then
  systemctl daemon-reload >/dev/null 2>&1 || true
fi
exit 0
POSTRM

chmod 0755 "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/prerm" "$PKG/DEBIAN/postrm"

INSTALLED_KB=$(du -sk "$PKG" | cut -f1)
cat > "$PKG/DEBIAN/control" <<CONTROL
Package: $PACKAGE
Version: $VERSION
Architecture: $ARCH
Maintainer: TDrive <tdrive@users.noreply.github.com>
Installed-Size: $INSTALLED_KB
Depends: ca-certificates
Recommends: ffmpeg, mpv
Section: net
Priority: optional
Description: Telegram-backed drive server with a web UI
 TDrive keeps files in a Telegram channel and serves them through a browser
 UI with streaming previews, uploads, downloads, and vault encryption.
CONTROL

rm -f "$OUT"
if [ "${TDRIVE_DEB_NO_DPKG:-}" != "1" ] && command -v dpkg-deb >/dev/null 2>&1; then
  if ! dpkg-deb --root-owner-group -b "$PKG" "$OUT" >/dev/null 2>&1; then
    dpkg-deb -b "$PKG" "$OUT" >/dev/null
  fi
else
  # COPYFILE_DISABLE stops macOS tar from adding ._ AppleDouble members.
  printf '2.0\n' > "$WORK/debian-binary"
  (
    cd "$PKG/DEBIAN"
    COPYFILE_DISABLE=1 tar -czf "$WORK/control.tar.gz" --exclude='.DS_Store' --exclude='._*' .
  )
  (
    cd "$PKG"
    COPYFILE_DISABLE=1 tar -czf "$WORK/data.tar.gz" --exclude='./DEBIAN' --exclude='.DS_Store' --exclude='._*' .
  )
  # Apple's ar writes a BSD archive with a symbol table, which dpkg rejects.
  # The ar container itself is simple enough to write directly: a magic line,
  # then one 60-byte header per member, data padded to an even offset.
  (
    cd "$WORK"
    printf '!<arch>\n'
    for member in debian-binary control.tar.gz data.tar.gz; do
      size=$(wc -c < "$member" | tr -d ' ')
      printf '%-16s%-12s%-6s%-6s%-8s%-10s`\n' "$member" "$(date +%s)" 0 0 100644 "$size"
      cat "$member"
      if [ $((size % 2)) -ne 0 ]; then printf '\n'; fi
    done
  ) > "$OUT"
fi

[ -s "$OUT" ] || die "no package was written"
printf '==> %s (%s bytes)\n' "$OUT" "$(wc -c < "$OUT" | tr -d ' ')"
ar t "$OUT" | sed 's/^/    /'
