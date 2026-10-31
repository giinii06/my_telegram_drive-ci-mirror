#!/usr/bin/env bash
# Builds TDrive-<version>.apk: the Go server as an executable native library
# plus the Kotlin foreground service that runs it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

command -v go >/dev/null 2>&1 || { echo "build-apk: go is required" >&2; exit 1; }
if [ -z "${ANDROID_HOME:-}${ANDROID_SDK_ROOT:-}" ]; then
  echo "build-apk: set ANDROID_HOME or ANDROID_SDK_ROOT to the Android SDK (platform 34, build-tools 34)" >&2
  exit 1
fi
gradle_bin="${GRADLE_BIN:-gradle}"
command -v "$gradle_bin" >/dev/null 2>&1 || {
  echo "build-apk: gradle 8.7 is required; set GRADLE_BIN to its path" >&2
  exit 1
}

version="${TDRIVE_VERSION:-}"
if [ -z "$version" ]; then
  version="$(git -C "$root" describe --tags --always --dirty 2>/dev/null || echo dev)"
fi
version_name="${TDRIVE_VERSION_NAME:-${version#v}}"
version_code="${TDRIVE_VERSION_CODE:-$(date +%Y%m%d)}"
commit="$(git -C "$root" rev-parse --short HEAD 2>/dev/null || echo unknown)"
ldflags="-s -w -X main.buildVersion=$version -X main.buildCommit=$commit"

# Go binary for each packaged ABI. They land in the app's nativeLibraryDir,
# which is the only app-owned path Android executes.
build_lib() {
  local abi="$1" goarch="$2"
  mkdir -p "$here/jniLibs/$abi"
  ( cd "$root" && CGO_ENABLED=0 GOOS=android GOARCH="$goarch" \
      go build -trimpath -ldflags "$ldflags" \
      -o "$here/jniLibs/$abi/libtdrive.so" ./cmd/tdrive )
  chmod 0755 "$here/jniLibs/$abi/libtdrive.so"
}

for abi in ${TDRIVE_ANDROID_ABIS:-arm64-v8a}; do
  case "$abi" in
    arm64-v8a) build_lib "$abi" arm64 ;;
    # android/amd64 needs cgo and an NDK, so it is not part of the default set.
    x86_64) build_lib "$abi" amd64 ;;
    *) echo "build-apk: unsupported ABI $abi" >&2; exit 1 ;;
  esac
done

( cd "$here" && "$gradle_bin" --no-daemon --console=plain \
    -PtdriveVersionName="$version_name" \
    -PtdriveVersionCode="$version_code" \
    assembleDebug )

apk="$here/app/build/outputs/apk/debug/app-debug.apk"
[ -f "$apk" ] || { echo "build-apk: gradle produced no APK" >&2; exit 1; }
mkdir -p "$root/dist"
out="$root/dist/TDrive-$version_name.apk"
cp "$apk" "$out"
echo "TDrive APK: $out"
