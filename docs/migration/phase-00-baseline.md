# Phase 00 — Go 기준선 동결 및 위험 조사

**목표**: Rust 전환을 판단할 현재 동작의 근거를 확보. 운영 코드 교체 없음.

## 분석/변경 위치

- README.md, README.dev.md, Taskfile.yml, go.mod, cmd/tdrive/*, cmd/tdrive-backup/*.
- backend/core, projection, crypto, tgclient, daemon, sync, media, mount*, auth, datadir, processlock.
- Dockerfile, docker-compose.yml, docker/entrypoint.sh, install.sh, packaging/*, .github/workflows/*.
- 신규 보고서: docs/migration/baseline-report.md, tests/fixtures/manifest.json, api-inventory 문서 (후속 MR).

## 세부 실행 항목

1. CLI help, commands --json, --drive-id/--json, exit code, stdout/stderr, default config 경로, daemon 시작과 락·종료 순서 표로 정리.
2. IPC v6의 request/frame/event/error 및 모든 명령과 HTTP routes/method/status/header/body를 스냅샷한다. 내부 WebDAV GET/PUT/MKCOL/PROPFIND 등 실제 지원 목록은 코드+테스트로 확정.
3. SQLite schema_version 16, PRAGMA, 테이블/인덱스/트리거/FK, 개인 채널 migration, 회복 경로와 암호화 상태의 경로/파일 모드 조사.
4. 저장 데이터: TDX1, TDE1, name v1 envelope, Telegram gotd session, backup JSON journal, processlock, web token, vault.pass 샘플은 비밀 제거 fixture로 생성.
5. 기존 Go 테스트/빌드와 Bun/ESLint/Playwright 및 Docker/설치 smoke test를 실제 CI에서 실행하고 SHA·로그·아키텍처를 보관.
6. 불안정 시나리오: FLOOD_WAIT, timeout after remote accept, bad access_hash, offline, cancellation, DB failure, power loss between send/project, partial upload, mount stale cache, duplicate filename, 2FA invalid, corrupted journal.
7. 기준선: 100/10k/100k entries, 100MB/1GB 테스트 파일, concurrent range reads, cold-start/sync latency, idle/peak RSS/CPU, resume time. 환경 및 반복 횟수와 결과 통계를 기재.
8. 네이티브 설치별 지원/업데이트·삭제 시 config 보존, Android 호스트 WebView, Windows 백업 작업 스케줄을 표로 만든다.

## 검증할 명령 (현재 MR에서 실행한 사실 아님)

~~~sh
go test ./cmd/tdrive ./backend/...
go vet ./cmd/tdrive ./backend/...
go build -trimpath -o /tmp/tdrive-legacy ./cmd/tdrive
bun install --cwd cmd/tdrive/webui --frozen-lockfile
bun run --cwd cmd/tdrive/webui lint
bun run --cwd cmd/tdrive/webui test:unit
bun run --cwd cmd/tdrive/webui test:browser
bash -n install.sh
TDRIVE_DRY_RUN=1 bash install.sh
~~~

## 완료 조건

- [ ] API/IPC/CLI/OS mount 전체 inventory와 누락 확인
- [ ] 비밀 없는 fixture manifest + sample SHA + generator 확보
- [ ] 기존 Go build/test/Playwright/installer 결과 및 실패 이유 기록
- [ ] 사용자 데이터·계정 취급 없는 재현성 있는 장애 테스트
- [ ] 플랫폼별 기능 지원 매트릭스 및 위험 owner 작성
- [ ] 수치 조작 없는 성능 기준선 생성

**실패/롤백**: 테스트 중 실계정이나 운영 데이터가 필요하면 중단하고 별도 sandbox로 대체. secret fixture 유출 시 즉시 공유 중지/재생성. 코드 변경은 아직 불허.
