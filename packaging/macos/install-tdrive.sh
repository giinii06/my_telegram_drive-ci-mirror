#!/usr/bin/env bash
# Installs the TDrive binary and starts it as a per-user LaunchAgent. Safe to
# re-run: the previous agent is unloaded before the new one is bootstrapped.
set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LABEL="com.tdrive.web"
PORT="${TDRIVE_PORT:-8080}"
INSTALL_DIR="$HOME/Applications/TDrive/bin"
LAUNCH_AGENTS="$HOME/Library/LaunchAgents"
PLIST="$LAUNCH_AGENTS/$LABEL.plist"
LOG_DIR="$HOME/Library/Logs"
LOG_FILE="$LOG_DIR/TDrive-web.log"
TOKEN_FILE="$HOME/.config/TDrive/web.token"

info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

[ "$(uname -s)" = "Darwin" ] || die "이 설치 프로그램은 macOS 전용입니다."
[ -f "$SRC/tdrive" ] || die "tdrive 실행 파일이 없습니다. dmg를 다시 열어 실행하세요."

info "TDrive를 $INSTALL_DIR 에 설치합니다"
mkdir -p "$INSTALL_DIR" "$LAUNCH_AGENTS" "$LOG_DIR"
install -m 755 "$SRC/tdrive" "$INSTALL_DIR/tdrive"
install -m 755 "$SRC/open-tdrive.sh" "$INSTALL_DIR/open-tdrive.sh"
install -m 755 "$SRC/Open TDrive.command" "$INSTALL_DIR/Open TDrive.command"
install -m 755 "$SRC/install-tdrive.sh" "$INSTALL_DIR/install-tdrive.sh"

info "로그인 시 자동 실행되는 LaunchAgent를 등록합니다"
cat > "$PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>$INSTALL_DIR/tdrive</string>
    <string>web</string>
    <string>--listen</string>
    <string>127.0.0.1</string>
    <string>--port</string>
    <string>$PORT</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>TDRIVE_NO_OS_MOUNT</key>
    <string>1</string>
    <key>PATH</key>
    <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>$LOG_FILE</string>
  <key>StandardErrorPath</key>
  <string>$LOG_FILE</string>
</dict>
</plist>
PLIST_EOF

plutil -lint "$PLIST" >/dev/null || die "LaunchAgent plist가 올바르지 않습니다: $PLIST"

launchctl bootout "gui/$UID/$LABEL" >/dev/null 2>&1 || \
  launchctl unload "$PLIST" >/dev/null 2>&1 || true
if ! launchctl bootstrap "gui/$UID" "$PLIST" >/dev/null 2>&1; then
  launchctl load -w "$PLIST" >/dev/null 2>&1 || warn "LaunchAgent를 직접 로드하지 못했습니다. 다음 로그인 때 자동으로 시작됩니다."
fi

info "웹 UI 주소를 준비하는 중입니다 (최대 15초)"
for _ in $(seq 1 30); do
  [ -s "$TOKEN_FILE" ] && break
  sleep 0.5
done

if [ ! -s "$TOKEN_FILE" ]; then
  warn "아직 토큰 파일이 없습니다: $TOKEN_FILE"
  warn "로그를 확인하세요: $LOG_FILE"
  exit 0
fi

info "설치 완료"
printf '  웹 UI:   ./Open TDrive.command 또는 %s/Open TDrive.command\n' "$INSTALL_DIR"
printf '  로그:    %s\n' "$LOG_FILE"
printf '  중지:    launchctl bootout gui/$UID/%s\n' "$LABEL"
printf '  시작:    launchctl bootstrap gui/$UID %s\n' "$PLIST"

exec "$SRC/open-tdrive.sh"
