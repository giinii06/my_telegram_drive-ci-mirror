#!/usr/bin/env bash
# TDrive bootstrap: Docker Compose only. This script never installs a host
# TDrive binary; the CLI always runs inside the container.
set -euo pipefail

REPO_URL="${TDRIVE_REPO:-https://github.com/giinii06/my_telegram_drive.git}"
REF="${TDRIVE_REF:-main}"
TDRIVE_DIR="${TDRIVE_HOME:-$HOME/.tdrive}"
BIN_DIR="${TDRIVE_BIN_DIR:-$HOME/.local/bin}"
DRY_RUN="${TDRIVE_DRY_RUN:-0}"
IMAGE="${TDRIVE_IMAGE:-ghcr.io/giinii06/my_telegram_drive:latest}"
USE_PREBUILT=0
if [ -n "${TDRIVE_IMAGE:-}" ]; then
  USE_PREBUILT=1
else
  case "$(uname -m)" in
    x86_64|amd64) USE_PREBUILT=1 ;;
  esac
fi

info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

has_tty() { (exec </dev/tty) 2>/dev/null; }

read_secret_tty() {
  local prompt="$1" var="$2" value=""
  has_tty || return 1
  printf '%s' "$prompt" >/dev/tty
  IFS= read -rs value </dev/tty || return 1
  printf '\n' >/dev/tty
  printf -v "$var" '%s' "$value"
}

compose() {
  docker compose --env-file "$TDRIVE_DIR/.env" -f "$TDRIVE_DIR/docker-compose.yml" "$@"
}

require_linux() {
  [ "$(uname -s)" = "Linux" ] || die "Linux 서버에서 실행하세요. 현재 compose는 /dev/net/tun이 필요한 Tailscale 사이드카를 사용합니다."
}

require_tun() {
  [ -e /dev/net/tun ] || die "/dev/net/tun이 없습니다. TUN을 활성화한 뒤 다시 실행하세요: sudo modprobe tun"
}

require_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    if [ "${TDRIVE_INSTALL_DOCKER:-0}" != "1" ]; then
      die "Docker가 필요합니다. https://docs.docker.com/engine/install/ 를 참고하거나 TDRIVE_INSTALL_DOCKER=1로 다시 실행하세요."
    fi
    command -v curl >/dev/null 2>&1 || die "Docker 설치에는 curl이 필요합니다: sudo apt-get install -y curl"
    info "Docker 설치 (get.docker.com)"
    curl -fsSL https://get.docker.com | sh
    command -v docker >/dev/null 2>&1 || die "Docker 설치에 실패했습니다."
  fi
  docker info >/dev/null 2>&1 || die "Docker 데몬에 접근할 수 없습니다. sudo usermod -aG docker \$USER 후 다시 로그인하고 재실행하세요."
  docker compose version >/dev/null 2>&1 || die "Docker Compose v2가 필요합니다 (docker compose)."
}

require_git() {
  command -v git >/dev/null 2>&1 || die "git이 필요합니다: sudo apt-get install -y git"
}

sync_repo() {
  if [ -d "$TDRIVE_DIR/.git" ]; then
    info "저장소 업데이트: $TDRIVE_DIR"
    git -C "$TDRIVE_DIR" fetch --depth 1 origin "$REF"
    git -C "$TDRIVE_DIR" checkout -q FETCH_HEAD
    return 0
  fi
  if [ -e "$TDRIVE_DIR" ] && [ -n "$(ls -A "$TDRIVE_DIR" 2>/dev/null)" ]; then
    die "$TDRIVE_DIR 가 비어 있지 않고 git 저장소가 아닙니다. 경로를 옮기거나 비운 뒤 다시 실행하세요."
  fi
  info "저장소 복제: $TDRIVE_DIR"
  git clone --depth 1 --branch "$REF" "$REPO_URL" "$TDRIVE_DIR"
}

write_env() {
  local key=""
  if [ -f "$TDRIVE_DIR/.env" ]; then
    key="$(sed -n 's/^TS_AUTHKEY=//p' "$TDRIVE_DIR/.env" | tail -n 1)"
  fi
  if [ -z "$key" ]; then
    key="${TS_AUTHKEY:-}"
  fi
  if [ -z "$key" ]; then
    info "Tailscale auth key가 필요합니다: https://login.tailscale.com/admin/settings/keys"
    read_secret_tty "Tailscale auth key: " key || die "Tailscale auth key를 읽지 못했습니다."
  fi
  [ -n "$key" ] || die "Tailscale auth key가 비어 있습니다."
  case "$key" in
    tskey-*) ;;
    *) warn "Tailscale auth key는 보통 tskey-로 시작합니다." ;;
  esac
  umask 077
  {
    printf 'TS_AUTHKEY=%s\n' "$key"
    printf 'TDRIVE_IMAGE=%s\n' "$IMAGE"
    printf 'COMPOSE_PROJECT_NAME=tdrive\n'
  } > "$TDRIVE_DIR/.env"
}

start_stack() {
  if [ "$USE_PREBUILT" = "1" ]; then
    if compose pull tdrive; then
      info "이미지 다운로드 완료: $IMAGE"
      compose up -d
      return 0
    fi
    warn "이미지를 내려받지 못했습니다. 로컬에서 빌드합니다 (몇 분 걸릴 수 있습니다)."
  else
    info "이 아키텍처($(uname -m))에는 배포 이미지가 없어 로컬에서 빌드합니다 (몇 분 걸릴 수 있습니다)."
  fi
  compose up -d --build
}

wait_ready() {
  local i
  info "데몬 준비 대기"
  for i in $(seq 1 60); do
    if compose exec -T tdrive tdrive daemon status >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  die "컨테이너가 준비되지 않았습니다. 로그 확인: docker compose -f \"$TDRIVE_DIR/docker-compose.yml\" logs tdrive"
}

install_wrapper() {
  mkdir -p "$BIN_DIR"
  cat > "$BIN_DIR/tdrive" <<'WRAPPER'
#!/bin/sh
set -eu

home="${TDRIVE_HOME:-$HOME/.tdrive}"
file="$home/docker-compose.yml"
if [ ! -f "$file" ]; then
  echo "tdrive: $file not found; run the installer first" >&2
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  echo "tdrive: Docker daemon is not reachable" >&2
  exit 1
fi

compose() {
  docker compose --env-file "$home/.env" -f "$file" "$@"
}

running="$(compose ps -q tdrive 2>/dev/null || true)"
if [ -z "$running" ]; then
  compose up -d >/dev/null
fi

if [ -t 0 ] && [ -t 1 ]; then
  compose exec tdrive tdrive "$@"
else
  compose exec -T tdrive tdrive "$@"
fi
WRAPPER
  chmod +x "$BIN_DIR/tdrive"
}

report() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) warn "$BIN_DIR 가 PATH에 없습니다. 셸 설정에 추가하세요: export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
  esac
  local ip="" i
  for i in $(seq 1 30); do
    ip="$(compose exec -T tailscale tailscale ip -4 2>/dev/null | head -n 1 || true)"
    [ -n "$ip" ] && break
    sleep 1
  done
  printf '\n설치 완료\n\n'
  printf '  tdrive status\n'
  printf '  tdrive drives\n'
  printf '  tdrive help\n\n'
  printf '브라우저에서 Telegram API ID/hash, 전화번호, 로그인 코드, 2FA를 입력하세요.\n'
  if [ -n "$ip" ]; then
    printf '웹: http://%s:8080 (Tailscale에 연결된 기기에서, 로그인 없음)\n' "$ip"
  else
    printf '웹: Tailscale 연결 후 http://<tailscale-ip>:8080\n'
  fi
}

main() {
  if [ "$DRY_RUN" = "1" ]; then
    info "dry run: 변경 없이 계획만 표시합니다"
    if [ "$USE_PREBUILT" = "1" ]; then
      mode="pull 우선"
    else
      mode="로컬 빌드"
    fi
    printf '  설치 경로: %s\n  저장소: %s (%s)\n  래퍼: %s/tdrive\n' "$TDRIVE_DIR" "$REPO_URL" "$REF" "$BIN_DIR"
    printf '  이미지: %s (%s)\n' "$IMAGE" "$mode"
    printf '  단계: clone/update -> .env -> image -> web onboarding -> wrapper\n'
    return 0
  fi
  require_linux
  require_tun
  require_docker
  require_git
  sync_repo
  write_env
  start_stack
  wait_ready
  install_wrapper
  report
}

main "$@"
