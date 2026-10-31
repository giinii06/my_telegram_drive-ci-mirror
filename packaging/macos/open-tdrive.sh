#!/usr/bin/env bash
# Opens the local TDrive UI with the access token from the token file.
set -euo pipefail

PORT="${TDRIVE_PORT:-8080}"
TOKEN_FILE="$HOME/.config/TDrive/web.token"
URL="http://127.0.0.1:$PORT/"

if [ ! -s "$TOKEN_FILE" ]; then
  printf 'error: 토큰 파일이 없습니다: %s\n' "$TOKEN_FILE" >&2
  printf 'TDrive가 실행 중인지 확인하세요.\n' >&2
  printf '  시작: launchctl kickstart -k gui/%s/com.tdrive.web\n' "$UID" >&2
  printf '  로그: %s\n' "$HOME/Library/Logs/TDrive-web.log" >&2
  exit 1
fi

TOKEN="$(tr -d '\n' < "$TOKEN_FILE")"
if [ -z "$TOKEN" ]; then
  printf 'error: 토큰 파일이 비어 있습니다: %s\n' "$TOKEN_FILE" >&2
  exit 1
fi

open "$URL?token=$TOKEN"
