# 현재 저장소 구조와 이행 위험 지도

## 조사 근거

아래는 main SHA 7edfea5456f4e9b18305e15e9de5d149976331da의 GitLab 트리 및 지정 소스 내용을 읽은 결과다. 추후 기준 SHA가 바뀌면 실제 코드 확인 후 이 표를 업데이트해야 한다. 실서버 데이터, 실제 Telegram 계정, 실행 환경의 동작은 이 문서 작성 시 테스트하지 않았다.

| 영역 | 현재 구현/위치 | 의미와 이행 위험 |
|---|---|---|
| 엔트리포인트 | cmd/tdrive/main.go, cmd/tdrive/web.go | CLI가 데몬을 자동 시작. web은 동일 데몬 호출. |
| 코어 DI | backend/core/engine.go | Telegram client, services, live sync, 파일/미디어 구성. Go→Rust 의존성 전환 중심. |
| 사용자 Telegram | backend/auth/auth.go, backend/auth/session_storage.go, backend/tgclient/{client.go,gotd.go,gotd_pool.go,write_coordinator.go} | gotd 기반 인증·세션과 DC 풀, 안정 random_id, FLOOD_WAIT. Bot API로 대체할 수 없음. |
| 영속성 | backend/fstructs.go, backend/projection/{schema.go,writable_schema.go,log.go} | 단일 SQLite 연결 및 스키마 마이그레이션, replay_log의 SHA-256. |
| 메시지 형식 | backend/projection/{types.go,wire.go,apply.go} | TDX1, 폴더 ID d:, 파일 ID f:, root parent 빈 문자열. 과거 op는 영구 호환. |
| 동기화 | backend/sync/{sync.go,parse.go,initial_scan_spool.go,difference.go}, backend/livesync/* | 최초·증분 스캔, 순서 있는 replay, backstop. |
| 암호화 | backend/crypto/{vault.go,stream.go,random_access.go,name.go} | TDE1(50-byte header, 64KiB chunks), Argon2id, XChaCha20-Poly1305, HKDF-SHA256. |
| 안전 정책 | backend/mountpolicy/policy.go, backend/core/engine.go | 암호화 여부 불확실하면 평문 쓰기 거부; vault 잠금 시 세션 세대 격리. |
| 미디어 | backend/media/{service.go,range_reader.go,hls.go,remux/*} | Telegram range 다운로드, 토큰별 media 세션, MKV→fMP4/HLS 재인코딩 없는 리먹싱. |
| 마운트 | backend/mount{adapter,cache,content,controller,dav,fs,lifecycle,os,path,policy,safe,scratch,write}/* | WebDAV, host OS 연계, 쓰기 journal과 mount state. 컨테이너에서는 TDRIVE_NO_OS_MOUNT=1. |
| IPC | backend/daemon/{protocol.go,server_dispatch.go,socket_unix.go,socket_windows.go} | ProtocolVersion=6, Unix socket 0600·사용자별 Windows named pipe ACL. |
| 브라우저 UI | cmd/tdrive/webui/index.html, package.json, tests/*.mjs | 하나의 HTML에 Svelte 기반 앱 포함; Bun 1.4.3, Playwright. GPUI 도입과 별개의 사용자 표면. |
| 웹 인증/파일 API | cmd/tdrive/web.go, cmd/tdrive/web_proxy.go, cmd/tdrive/web_onboarding.go | 로컬 토큰·Tailscale no-auth 모드·온보딩·업로드 큐·WebDAV 프록시. |
| 백업 | cmd/tdrive-backup/{main.go,state.go,client.go} | 원본을 **원격 검증 후** 삭제하는 위험한 계약, JSON journal. |
| Docker | Dockerfile, docker-compose.yml, docker/entrypoint.sh | Go 빌드, /data 볼륨, Tailscale 사이드카 네트워크 공유, Linux /dev/net/tun과 권한. |
| 설치/배포 | install.sh, packaging/{linux,macos,windows,android}/*, .github/workflows/* | deb/dmg/exe/APK, Go 바이너리 기준. GitLab CI 설정 파일은 기준 트리에서 미확인. |

## 저장 형식에서 바로 지켜야 할 핵심

- **schema_version = 16**: backend/projection/schema.go. files/folders 생성은 개인 채널 마이그레이션 단계와 연결되어 있어 단순 CREATE TABLE 복제는 위험하다. replay_log, tamper/reject 로그, pending joins, revision, rendition, trash, writable op tables까지 조사한다.
- **TDX1**: backend/projection/wire.go + apply.go + writable_apply.go. (channel_id, msg_id) 오름차순, 멱등성, 누락된/삭제된 parent를 그대로 드러내는 정책과 cycle rejection을 보존한다.
- **TDE1**: backend/crypto/stream.go. 50바이트 header, 64 KiB chunk, 16바이트 AEAD tag, 마지막 청크 인증. Go 암호화 데이터를 Rust가 읽고 Rust 데이터를 Go가 읽어야 한다.
- **프로토콜 버전 6**: backend/daemon/protocol.go. 독립 설치된 구 CLI와 새 daemon 조합의 행동을 명시한다. silent fallback 금지.
- **세션 파일**: gotd 세션은 다른 Rust 라이브러리에서 무조건 직접 해석 가능하지 않다. 변환기가 준비되지 않은 상태에서는 재로그인이 더 안전할 수 있다. 사용자에게 기존 인증 파일을 삭제하라고 요구하지 않는다.
- **파일/미디어**: multipart manifest, writable revision, content_msg_id, replay log, tombstone/hard delete, checksum, range, token/capability URL을 보존한다.
- **프로세스 단일 작성자**: backend/processlock/lock.go는 PID 파일로 단일 백엔드를 보장. 컨테이너/호스트 혼합 운영에서 별도 locking 결정을 해야 한다.

## 확인된 특별 사례

1. backend/core/engine.go의 로컬 메시지 쓰기는 **Telegram send → SQLite project**로 원자적이지 않다. Telegram에 성공하고 로컬에 실패한 경우 다음 sync가 수렴하게 설계되어 있다. Rust가 DB 트랜잭션으로 네트워크 전송까지 묶는다고 가정하지 않는다.
2. backend/livesync/coordinator.go는 Telegram 이벤트를 변경 신호로만 취급하고 **이력 pull을 진실의 원천**으로 사용한다. debounce, periodic backstop, Stop/Start를 보존한다.
3. backend/crypto/vault.go 주석이 명시하는 v1 보안 경계: 원격 콘텐츠 기밀성에 초점; namespace/control 메타데이터 무결성·객체간 대체 방어를 과장하지 않는다.
4. docker/entrypoint.sh는 현재 vault.pass를 자동 읽어 unlock 시도한다. 신규 이미지에서는 **기본 자동 언락을 답습할지** 보안 ADR을 먼저 결정한다. 기존 자동 동작을 조용히 바꾸지 않는다.
5. README.md의 Linux Compose는 tailnet에서 **별도 HTTP 로그인 없음**이며 Tailscale 멤버십이 사실상의 경계다. 보안 강화가 필요하더라도 API 호환과 로그인 UX를 계획적으로 이행한다.
6. cmd/tdrive-backup/state.go는 잘못된 JSON journal을 격리한다. 새 백업 에이전트가 이를 빈 journal처럼 취급하면 중복 업로드·원본 삭제 위험이 있다.

## 즉시 위험도

| 위험 | 영향 | 우선 보호 |
|---|---|---|
| wire / schema 불일치 | 복원 불가·데이터 분실 | golden fixtures + 이중 방향 호환 |
| 암호화 알고리즘/바이트 스트림 차이 | 사용자 파일 해독 불가 | 사전 벡터 + 잘림·변조 테스트 |
| MTProto 라이브러리 의미 차이 | 중복 업로드·FloodWait·로그인 실패 | 전송/재시도 adapter contract |
| Go/Rust 동시 운용 | DB/원격 상태 충돌 | 프로세스 락 + 독립 데이터 디렉터리 |
| WebDAV/원격 파일 쓰기 | 잘린 파일·읽기 오류 | crash injection / atomic commit |
| UI vs 서버 배포 혼동 | headless 이미지 실행 실패 | GPUI 호스트 실행, 서버 Docker |
| 네이티브 설치 경로 차이 | 기존 config 유실 | 경로 mapping + 스냅샷/복원 |
| Android/Windows 지원 약화 | 기능 퇴행 | 플랫폼별 유지/이행 별도 승인 |

## 추가로 수집할 근거

- 전체 HTTP 경로와 response/error sample snapshot; 네이티브 설치별 환경변수·파일 경로; SQLite 실제 테이블·인덱스·FK dump.
- 실제 비밀을 제거한 기존 DB 사본, replay_log 표본, 다중 메시지/암호화 fixture, 대용량 파일, 원격 계정의 최소 권한 테스트 환경.
- 정확한 Go 단위 테스트 목록/CI 결과, 메모리·처리량·latency 기준선.
- 이 자료가 확보되기 전에는 스키마 버전 올리기·실제 세션 변환·Go 코드 삭제 금지.
