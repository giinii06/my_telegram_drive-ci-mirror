# Phase 15 — Shadow 검증, 점진 전환, 재난 복구, Go 퇴역 결정

**선행** Phase 00~14의 각 gate 통과 (지원 대상별 조건) / **결과** 데이터 무손실 Rust 서비스를 기본으로 전환하되 장애 시 확인된 Go 복귀 경로를 유지.

## 가장 중요한 규칙

**Go와 Rust는 하나의 데이터 디렉터리 또는 같은 Telegram 계정의 동일 writer로 동시에 실행되지 않는다.** Shadow는 snapshot/fixture를 읽는 **독립 Rust 환경**에서만 수행한다. Phase 15 이전에는 go.mod/backend/Go Dockerfile/native binary를 삭제하지 않는다.

## 착수 전 수집 항목

- 이전 Go release artifact, Go binary, Docker image digest, installer archive, config/schema docs, latest known-good backup.
- 사용자 데이터/SQLite(+WAL) consistent backup, checksum/manifest, current config/secret owner/permissions, Telegram channel IDs와 sync watermarks.
- Rust candidate artifact의 commit SHA, CI tests/P0 risk acceptance, supported-platform matrix, manual rollback runbook.
- 계정 session conversion 가능 여부 / 사용자 재로그인 단계와 vault unlock 안내.
- 장애 시 사용자에게 공지할 status/error/재시도 안내 및 책임자.

## 단계 A — 비파괴 Shadow / 읽기 검증

1. Go 운영 service는 기존대로 유지; read-only snapshot/fixture를 별도 Rust sandbox에 넣는다. Rust sandbox는 **Telegram remote 쓰기 및 운영 계정 로그인 금지**.
2. 모든 channel을 Go replay와 Rust replay로 독립 재구축하고 normalized table counts/hash, file tree, tombstones, revisions, encryption flags, multipart map 비교.
3. TDX1, TDE1, API/IPC/CLI snapshots, media range/HLS, backup journal, UI 동작을 replay. 10k/100k entries perf, ARM64/AMD64 load, memory and handle leaks 측정.
4. mismatches: P0 우선 해결, expected differences는 ADR·test로 명시. 잘못된 row count/crypto byte 차이가 있으면 다음 단계 차단.

## 단계 B — 테스트 계정 Canary

1. 격리 Telegram 계정/채널과 새로운 데이터 디렉터리를 Rust 단독으로 운영.
2. 새 파일 업로드/rename/move/복사/복원, vault, large multipart, failure/retry, backup client, HLS, writable mount, GPUI, docker 및 remote Tailscale 연결을 E2E.
3. deliberate faults: FloodWait, packet loss, power loss, storage full, daemon crash, stale hash, vault lock, proxy replay, sidecar restart.
4. Go client와 Rust client의 순차 호환(같은 채널 데이터를 읽는 시간은 분리) 확인, unknown-op 처리, session conversion/re-login UX 문서화.

## 단계 C — 제한적 실제 사용자 전환

1. 일정/휴지시간 공지 후 사용자가 승인한 설치에서만 Go daemon/backup upload 중지; 큐 drain, single-writer/processlock 확인.
2. DB+WAL+config+session consistent backup 및 복원 테스트. 디스크 용량과 secret permissions 확인.
3. 복제본에서 Rust migration dry run + 검증 후 실제 activation. upgrade log에는 비밀/파일명/채널 메시지 내용 포함 금지.
4. Rust daemon startup, /api/status, DB integrity, Tailscale reachability, onboarding/재로그인/vault unlock, browse/upload/download/media/mount/backup 등 기본 smoke.
5. 일정 기간 conservative flags: 읽기 먼저 → 신규 업로드 → rename/move → 백업 자동삭제 → writable mount → hard delete. 각 단계에 별도 승인.
6. Go compatible client path 유지; 플랫폼별 rollout 분리: Linux Docker 먼저, macOS GPUI/네이티브 다음, Windows/Android는 실제 지원 상황에 따라 독립 결정.
7. 사용자 관측: panic/crash, API 5xx, auth failure, replay divergence, upload duplicate count, lock race, corrupted data, CPU/RSS/p95, volume errors.

## 단계 D — 기본 전환 및 퇴역 판단

- 차등 검증/카나리/업그레이드 테스트를 마친 대상 플랫폼만 default Rust로 변경.
- Go 코드 삭제는 전체 플랫폼 지원/버전 데이터 호환, 백업 에이전트/모바일 유지 계획, 법적 라이선스 검토와 별도 irreversible-removal MR로.
- 이전 버전 사용자 공지를 위해 구 Docker image/dmg/exe/APK/Go 바이너리와 복원 runbook은 보관.
- 사용되지 않는 Go path라는 이유만으로 Telegram legacy op decoder/crypto migration fixture를 삭제하지 않는다.
- 프로토콜의 future versioning, graceful downgrade, security patches owner 지정.

## Rollback runbook (일반 틀; 환경별 상세 검증 필수)

~~~text
1. Incident severity 확인, write feature flags off, backup agent 원본 삭제 stop
2. Rust service graceful stop (필요하면 force stop), DB/process lock release 확인
3. Rust activation 이후 원격 Telegram에 기록된 message 목록 및 watermarks 기록
4. Go와 호환되는 schema인지 검사; 불호환이면 pre-upgrade consistent DB backup 복원
5. Go binary/image/compose/installer의 검증된 버전으로 실행
6. Go incremental sync를 사용해 그동안 원격에 기록된 op를 다시 가져오고 파일 트리 수렴 검증
7. vault unlock, HTTP access, file/hash, backup journal/local original 점검
8. 사용자의 손상/누락·중복 상태를 audit에 기록, 재배포 중단·사후 분석
~~~

**중요**: DB snapshot을 복원해도 그 이후 Telegram에 정상 전송된 remote ops는 되돌아가지 않는다. 따라서 해당 메시지를 무조건 삭제하거나 rollback이 완전한 시간 역행이라고 주장해서는 안 된다.

## Stop-the-line 조건

- 사용자 암호화 파일을 해독할 수 없거나 vault policy fail-closed 미작동.
- replay/projection/SQLite data loss, duplicate destructive remote write, backup source premature delete.
- 권한 없이 file/secret/token 노출, tailnet public exposure, Unix socket/named-pipe ACL 위반.
- long-lived lock contention, SIGTERM/crash 후 복구 불가능한 upload state, 실제 운영 data migration fail.
- CI 미실행·runner 없음·플랫폼 빌드 미검증을 통과로 대체하려는 경우.

## 최종 완료 정의

- [ ] Go↔Rust contract diff P0=0 및 승인된 expected diffs만 남음
- [ ] 읽기/쓰기/복구/crash/backup/local+remote install/cross platform tests
- [ ] 데이터 백업·복원 실험 성공, downgrade runbook operator 확인
- [ ] 사용자 기능 parity 및 지원 matrix, Windows/Android 잔여 계획
- [ ] 배포 지표에서 안전성 및 성능 기준선 충족
- [ ] 운영/지원/보안 owner와 이후 릴리스 프로세스 명시
- [ ] Go 제거는 **새로운 후속 MR**로만, 별도 승인 후

**중단/롤백**: 어떠한 데이터 유실 의심만 있어도 일괄 롤아웃 중단. 기능 차단/Go 복구/원격 수렴 검증이 끝날 때까지 재전환 금지.
