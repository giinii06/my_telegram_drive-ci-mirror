#!/usr/bin/env bash
# TDrive bootstrap: Docker Compose only. This script never installs a host
# TDrive binary; the CLI always runs inside the container. The image is always
# built locally: no registry pull, so GHCR visibility or auth never matters.
set -euo pipefail

REPO_URL="${TDRIVE_REPO:-https://github.com/giinii06/my_telegram_drive.git}"
REF="${TDRIVE_REF:-main}"
TDRIVE_DIR="${TDRIVE_HOME:-$HOME/.tdrive}"
BIN_DIR="${TDRIVE_BIN_DIR:-$HOME/.local/bin}"
DRY_RUN="${TDRIVE_DRY_RUN:-0}"
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

prompt_value() {
  local label="$1" current="$2" var="$3" value=""
  printf '%s [%s]: ' "$label" "$current" >/dev/tty
  IFS= read -r value </dev/tty || die "입력을 읽지 못했습니다."
  if [ -n "$value" ]; then printf -v "$var" '%s' "$value"; fi
  return 0
}

prompt_secret() {
  local label="$1" current="$2" var="$3" value=""
  printf '%s [%s]: ' "$label" "$current" >/dev/tty
  IFS= read -rs value </dev/tty || die "입력을 읽지 못했습니다."
  printf '\n' >/dev/tty
  if [ -n "$value" ]; then printf -v "$var" '%s' "$value"; fi
  return 0
}

mask_secret() {
  local value="$1"
  if [ -z "$value" ]; then
    printf '(없음)'
  elif [ "${#value}" -le 12 ]; then
    printf '********'
  else
    printf '%s…%s' "${value:0:6}" "${value: -4}"
  fi
}

compose() {
  docker compose --env-file "$TDRIVE_DIR/.env" -f "$TDRIVE_DIR/docker-compose.yml" "$@"
}

env_get() {
  local key="$1" file="$TDRIVE_DIR/.env"
  [ -f "$file" ] || return 0
  sed -n "s/^${key}=//p" "$file" | tail -n 1
}

env_set() {
  local key="$1" value="$2" file="$TDRIVE_DIR/.env" tmp
  mkdir -p "$TDRIVE_DIR"
  tmp="$(mktemp "$TDRIVE_DIR/.env.XXXXXX")"
  if [ -f "$file" ]; then
    grep -v "^${key}=" "$file" >"$tmp" || true
  fi
  printf '%s=%s\n' "$key" "$value" >>"$tmp"
  chmod 600 "$tmp"
  mv "$tmp" "$file"
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
  key="$(env_get TS_AUTHKEY)"
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
  env_set TS_AUTHKEY "$key"
  env_set COMPOSE_PROJECT_NAME tdrive
  # Older installs pinned the GHCR image here; drop it so compose builds locally.
  case "$(env_get TDRIVE_IMAGE)" in
    ""|ghcr.io/*) env_set TDRIVE_IMAGE tdrive:latest ;;
  esac
}

start_stack() {
  info "이미지를 로컬에서 빌드합니다 (레지스트리에서 내려받지 않습니다, 몇 분 걸릴 수 있습니다)"
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

cmd_onboard() {
  if [ "$DRY_RUN" = "1" ]; then
    printf 'onboard 대상: %s\n' "$TDRIVE_DIR/.env"
    printf '  키: TS_AUTHKEY, TDRIVE_PORT, TDRIVE_IMAGE, 추가 KEY=VALUE\n'
    printf '  이후: docker compose up -d --build\n'
    return 0
  fi
  [ -f "$TDRIVE_DIR/docker-compose.yml" ] || die "$TDRIVE_DIR/docker-compose.yml 가 없습니다. 설치를 먼저 실행하세요."
  has_tty || die "onboard는 터미널에서 실행해야 합니다."
  require_linux
  require_tun
  require_docker

  local key port image before_key extra name value
  before_key="$(env_get TS_AUTHKEY)"
  key="$before_key"
  port="$(env_get TDRIVE_PORT)"
  [ -n "$port" ] || port="8080"
  image="$(env_get TDRIVE_IMAGE)"
  [ -n "$image" ] || image="tdrive:latest"

  info ".env 값 입력 ($TDRIVE_DIR/.env) — Enter만 누르면 기존 값 유지"
  prompt_secret "Tailscale auth key (tskey-...)" "$(mask_secret "$key")" key
  prompt_value "웹 포트 (TDRIVE_PORT)" "$port" port
  prompt_value "이미지 태그 (TDRIVE_IMAGE)" "$image" image

  [ -n "$key" ] || die "Tailscale auth key가 비어 있습니다."
  case "$key" in
    tskey-*) ;;
    *) warn "Tailscale auth key는 보통 tskey-로 시작합니다." ;;
  esac
  case "$port" in
    ''|*[!0-9]*) die "TDRIVE_PORT는 숫자여야 합니다: $port" ;;
  esac
  if [ "$port" -lt 1 ] || [ "$port" -gt 65535 ]; then
    die "TDRIVE_PORT 범위는 1-65535입니다: $port"
  fi
  [ -n "$image" ] || die "TDRIVE_IMAGE가 비어 있습니다."
  case "$image" in
    *[[:space:]]*) die "TDRIVE_IMAGE에 공백이 있습니다: $image" ;;
  esac

  umask 077
  env_set TS_AUTHKEY "$key"
  env_set TDRIVE_PORT "$port"
  env_set TDRIVE_IMAGE "$image"

  info "추가 값: KEY=VALUE 형식으로 입력하고, 비우면 종료합니다"
  while :; do
    printf '추가 KEY=VALUE: ' >/dev/tty
    IFS= read -r extra </dev/tty || break
    [ -n "$extra" ] || break
    case "$extra" in
      *=*) ;;
      *) warn "KEY=VALUE 형식이 아닙니다: $extra"; continue ;;
    esac
    name="${extra%%=*}"
    value="${extra#*=}"
    case "$name" in
      ''|*[!A-Za-z0-9_]*) warn "올바르지 않은 키입니다: $name"; continue ;;
    esac
    env_set "$name" "$value"
    info "$name 저장"
  done

  info ".env 저장 완료"
  info "스택 재시작 (이미지 로컬 빌드)"
  compose up -d --build
  if [ "$key" != "$before_key" ]; then
    info "Tailscale 키가 바뀌어 tdrive 컨테이너를 다시 만듭니다"
    compose up -d --force-recreate tdrive
  fi
  wait_ready

  printf '\nonboard 완료\n\n'
  printf '  tdrive onboard   # 값 다시 수정\n'
  printf '  tdrive status\n\n'
  printf '웹: '
  web_url
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

if [ "${1:-}" = "onboard" ]; then
  if [ ! -f "$home/install.sh" ]; then
    echo "tdrive: $home/install.sh not found; re-run the installer" >&2
    exit 1
  fi
  TDRIVE_HOME="$home"
  export TDRIVE_HOME
  exec bash "$home/install.sh" onboard
fi

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

web_url() {
  local ip="" i port
  port="$(env_get TDRIVE_PORT)"
  [ -n "$port" ] || port="8080"
  for i in $(seq 1 30); do
    ip="$(compose exec -T tailscale tailscale ip -4 2>/dev/null | head -n 1 || true)"
    [ -n "$ip" ] && break
    sleep 1
  done
  if [ -n "$ip" ]; then
    printf 'http://%s:%s (Tailscale에 연결된 기기에서, 로그인 없음)\n' "$ip" "$port"
  else
    printf 'Tailscale 연결 후 http://<tailscale-ip>:%s\n' "$port"
  fi
}

report() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) warn "$BIN_DIR 가 PATH에 없습니다. 셸 설정에 추가하세요: export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
  esac
  printf '\n설치 완료\n\n'
  printf '  tdrive onboard   # .env 값 다시 입력\n'
  printf '  tdrive status\n'
  printf '  tdrive drives\n'
  printf '  tdrive help\n\n'
  printf '브라우저에서 Telegram API ID/hash, 전화번호, 로그인 코드, 2FA를 입력하세요.\n'
  printf '웹: '
  web_url
}

main() {
  case "${1:-}" in
    onboard)
      cmd_onboard
      return 0
      ;;
    "") ;;
    -h|--help|help)
      printf 'usage: install.sh [onboard]\n  onboard: .env 값을 다시 입력하고 스택을 재시작합니다.\n'
      return 0
      ;;
    *)
      die "알 수 없는 명령: $1 (usage: install.sh [onboard])"
      ;;
  esac
  if [ "$DRY_RUN" = "1" ]; then
    info "dry run: 변경 없이 계획만 표시합니다"
    printf '  설치 경로: %s\n  저장소: %s (%s)\n  래퍼: %s/tdrive\n' "$TDRIVE_DIR" "$REPO_URL" "$REF" "$BIN_DIR"
    printf '  이미지: tdrive:latest (로컬 빌드, pull 없음, %s)\n' "$(uname -m)"
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
