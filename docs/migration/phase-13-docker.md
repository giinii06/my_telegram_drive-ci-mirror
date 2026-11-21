# Phase 13 — Rust Docker 이미지 / Compose / Tailscale / 설치 경로

**선행** Phase 11 API and services parity / **결과** Linux headless Rust 이미지가 기존 Docker TDrive 사용자 데이터와 Tailnet 연결을 보존하며 안전하게 설치·재시작·복귀 가능.

## 현재 배포 현황 (원본 기준)

- Dockerfile: Go 1.25 multi-stage → Debian bookworm-slim + tini + /usr/local/bin/tdrive, /data volume.
- docker-compose.yml: tailscale sidecar (stable, /dev/net/tun, NET_ADMIN/SYS_MODULE, state volume), tdrive uses network_mode: service:tailscale, 8080 is not host-published by default.
- docker/entrypoint.sh: CLI daemon start, status wait, vault.pass optional auto unlock, web --listen 0.0.0.0 --no-auth, shutdown cleanup.
- install.sh: Docker Compose bootstrap, TDRIVE_HOME default ~/.tdrive, TDRIVE_BIN_DIR default ~/.local/bin, TS_AUTHKEY, local image build, CLI wrapper.
- .env.example, .dockerignore, README.md/README.dev.md.
- 현재 Compose 설계는 Linux /dev/net/tun에 의존. macOS/Windows Docker Desktop에서도 무조건 지원한다고 주장하지 않는다.

## 신규/수정 예상 파일 (후속 MR에서)

- docker/Dockerfile.rust (전환 전 별도), Dockerfile (검증 뒤 교체)
- docker/entrypoint-rust.sh, docker-compose.rust.yml (테스트 전용) → 검증 후 docker-compose.yml
- docker/healthcheck.sh, scripts/snapshot-and-upgrade.sh, scripts/restore.sh
- .dockerignore, .env.example, install.sh, docs/operations/docker.md
- apps/tdrive-server packaging, Cargo.lock, gitlab/github CI matrix
- **실제 사용자 .env, TS_AUTHKEY, volume은 Git에 저장하지 않는다.**

## 차례별 구현

1. **환경 조사**: Linux amd64/arm64, Oracle A1 ARM64 등 운영 타깃에서 rustc/deps/native build 가능성과 메모리 피크 측정. GPUI는 Docker binary graph에서 제외.
2. **멀티스테이지**: pinned rust build image/toolchain/Cargo.lock→cargo build --release --locked -p tdrive-server→minimal Debian runtime(or distroless) with ca-certs/tini and required dynamic libraries. static musl은 MTProto/media/SQLite native dependency 검증 뒤 결정.
3. **UID/GID**: /data의 기존 Go root-owned files를 Rust nonroot 사용자로 전환 시 발생할 권한 오류를 migration script가 식별/복원. Dockerfile에서 chown -R /data로 소유권을 조용히 변경하지 않는다. volume owner policy ADR.
4. **Runtime env**: HOME=/data, TDRIVE_CONTAINER=1, TDRIVE_NO_OS_MOUNT=1, PORT/TDRIVE_PORT, config/cache subdirs, log/UTC/locale, signal handling, health/readiness endpoint, resource limits.
5. **Entrypoint**: tini init, config check→daemon start→health readiness→web API start, SIGTERM graceful drain, DB close, upload worker shutdown. 자체 server single-process 방식으로 변경하는 경우 old background daemon behavior를 API 관점에서 보존.
6. **Tailscale network**: sidecar owns tun + NET_ADMIN, tdrive shares network namespace. TDrive 자체 컨테이너에 privileged/network caps 추가 금지. Tailscale state와 tdrive data **별도 volume** 보존.
7. **접근 범위**: tailnet-only by default, no published host port. 필요하면 명시적 loopback binding 예시. 기존 --no-auth는 tailnet membership 기반이므로 public 0.0.0.0 host publish와 동시에 사용 금지.
8. **Health**: API process is alive != Telegram login ready != configured drive != connected tailnet. 각각 liveness/readiness/degraded 상태 구분. healthcheck에서 secret/session URL 노출 금지.
9. **Persistence**: /data/.config/TDrive, .cache/TDrive, session, SQLite+WAL, vault data, secrets. Docker volume backup은 consistency snapshot으로. docker compose down -v는 여전히 데이터 삭제 위험이며 설치/업그레이드 도구가 실행 금지.
10. **Image builds**: linux/amd64+linux/arm64 test; OCI labels (git SHA/toolchain), SBOM, provenance, vulnerability/license scan. 외부 registry pull을 사용하지 않는 현행 install 정책 변경 시 ADR·사용자 안내 필수.
11. **Config migration**: 기존 .env 키와 명령 wrapper를 파싱/수용; TS_AUTHKEY rotation은 sidecar state와 함께 안전히. 무효 키/인터넷 중단에서도 저장 데이터 손상 없어야 한다.
12. **Upgrade dry-run**: 운영 Go compose를 중단하기 전 snapshot, 새 image는 **복제 볼륨**에서 migration+health+curl API parity, 실제 데이터는 복제본만 사용.
13. **Atomic cutover**: Go containers stop→data integrity/owner check→backup→new Compose up→health/onboard/auth/files/media/check→mark active. 실패 시 new stop→DB snapshot restore→Go compose up→Telegram incremental sync.
14. **Tailscale/restart recovery**: sidecar recreation, TDrive service restart, exit code, full host reboot, network reconnect, volumes retained; 서버가 tailnet 없이 잠시 offline이어도 data protected.
15. **Rootless/minimal caps 평가**: Tailscale userspace networking 등은 현재 기능이 동등함을 실증할 때만 대체. 성급히 tun 또는 policy를 제거하지 않는다.

## Compose target shape (설명용, 실행파일 아님)

~~~text
services:
  tailscale: existing sidecar, owned state volume, minimal required caps
  tdrive: Rust headless binary, network_mode service:tailscale, /data volume
  # no GPUI, no privileged GUI, no default publicly exposed host ports
volumes:
  tdrive-data: persistent
  tdrive-ts-state: persistent
~~~

## 테스트/장애 행렬

| 상황 | 기대 |
|---|---|
| Cold start with empty volumes | 웹 온보딩 가능, token/no-auth 설정 의도대로 |
| Upgrade from Go populated /data | files, vault, session or clear re-login, replay preserved |
| Wrong/expired TS_AUTHKEY | clear diagnosis, no data delete |
| Sidecar restart | daemon recover and retain auth/data |
| Rust image upgrade failure | Go image + consistent DB backup restoration |
| SIGTERM mid-upload | bounded draining, idempotent resume |
| Old volume UID/GID mismatch | fail with actionable error; no silent chmod |
| Docker CPU/RAM restricted | OOM diagnostics, bounded queue, no corrupt DB |
| linux/arm64 | image builds and runtime smoke on actual host |
| Tailnet absent/public port | prevent unsafe no-auth exposure |

## 완료 조건

- [ ] Linux AMD64/ARM64 image build+runtime+API tests
- [ ] 기존 Docker volume snapshot/upgrade/downgrade drill
- [ ] Tailscale tailnet-only + safe access model 검증
- [ ] nonroot/permission/secret handling and health signals
- [ ] install.sh wrapper/commands/.env backward compatibility
- [ ] data retention under reboot/restart/failed upgrade, no down -v

**롤백**: 반드시 새 Rust container 종료 및 DB writer lock 해제 후 backup/Go restore. Tailscale 볼륨과 auth state를 임의 초기화하지 않는다.
