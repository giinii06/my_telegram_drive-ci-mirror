#!/usr/bin/env bash
# Builds a universal (arm64 + amd64) macOS binary and packages it as a disk image.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION=""
APP_NAME="TDrive"

info() { printf '==> %s\n' "$*"; }
die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

usage() {
  printf 'usage: packaging/macos/build-dmg.sh [--version X]\n'
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || die "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1 (usage: build-dmg.sh [--version X])"
      ;;
  esac
done

[ "$(uname -s)" = "Darwin" ] || die "the dmg can only be built on macOS (hdiutil and lipo are required)"
command -v go >/dev/null 2>&1 || die "go is required"
command -v hdiutil >/dev/null 2>&1 || die "hdiutil is required"
command -v lipo >/dev/null 2>&1 || die "lipo is required (install the Xcode command line tools)"

if [ -z "$VERSION" ]; then
  VERSION="$(git -C "$ROOT" describe --tags --always --dirty 2>/dev/null || true)"
fi
[ -n "$VERSION" ] || VERSION="dev"
VERSION="${VERSION#v}"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || true)"
[ -n "$COMMIT" ] || COMMIT="unknown"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/tdrive-dmg.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

LDFLAGS="-s -w -X main.buildVersion=$VERSION -X main.buildCommit=$COMMIT"

info "building tdrive $VERSION ($COMMIT) for darwin/arm64 and darwin/amd64"
mkdir -p "$WORK/arm64" "$WORK/amd64"
( cd "$ROOT" && CGO_ENABLED=0 GOOS=darwin GOARCH=arm64 go build -trimpath -ldflags "$LDFLAGS" -o "$WORK/arm64/tdrive" ./cmd/tdrive )
( cd "$ROOT" && CGO_ENABLED=0 GOOS=darwin GOARCH=amd64 go build -trimpath -ldflags "$LDFLAGS" -o "$WORK/amd64/tdrive" ./cmd/tdrive )

STAGE="$WORK/stage"
mkdir -p "$STAGE"
lipo -create -output "$STAGE/tdrive" "$WORK/arm64/tdrive" "$WORK/amd64/tdrive"
chmod 755 "$STAGE/tdrive"

if command -v codesign >/dev/null 2>&1; then
  # Ad-hoc signing keeps Gatekeeper from treating the binary as damaged; it is
  # not a Developer ID signature, so the first launch still needs approval.
  codesign --force --sign - "$STAGE/tdrive" >/dev/null 2>&1 || true
fi

cp "$ROOT/packaging/macos/install-tdrive.sh" "$STAGE/install-tdrive.sh"
cp "$ROOT/packaging/macos/open-tdrive.sh" "$STAGE/open-tdrive.sh"
cp "$ROOT/packaging/macos/Install TDrive.command" "$STAGE/Install TDrive.command"
cp "$ROOT/packaging/macos/Open TDrive.command" "$STAGE/Open TDrive.command"
cp "$ROOT/packaging/macos/README.md" "$STAGE/README.txt"
chmod 755 "$STAGE/install-tdrive.sh" "$STAGE/open-tdrive.sh" \
  "$STAGE/Install TDrive.command" "$STAGE/Open TDrive.command"

"$STAGE/tdrive" version >/dev/null || die "the built universal binary does not run on this Mac"

mkdir -p "$ROOT/dist"
OUT="$ROOT/dist/${APP_NAME}-${VERSION}-universal.dmg"
info "creating $OUT"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$OUT" >/dev/null

info "done: $OUT"
