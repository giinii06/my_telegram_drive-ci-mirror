# Phase 11 — Rust Daemon / IPC / HTTP API / CLI / 기존 웹 UI 연계

**선행** Phase 06~10 / **결과** Rust 서버와 CLI가 기존 Go 서비스/웹 UX 계약을 충족. GPUI와 Docker가 소비할 안정된 API를 제공.

## 현재 근거

- backend/daemon/{protocol.go,server.go,server_dispatch.go,server_connection.go,server_commands.go,server_session.go,events.go,socket_unix.go,socket_windows.go}
- backend/core/engine.go, backend/processlock/lock.go
- cmd/tdrive/{main.go,machine.go,options.go,web.go,web_ui.go,web_onboarding.go,web_proxy.go,web_thumbnail.go,daemon_process.go}
- cmd/tdrive/webui/{index.html,package.json,tests/*.mjs}
- cmd/tdrive-backup/client.go: 동일 HTTP API를 소비한다.
- 현행 ProtocolVersion=6. Upgrade 시 구 CLI가 새 daemon과 섞이는 상황을 별도 고려해야 한다.

## 목표 신규 코드

- apps/tdrive-server/src/{main.rs,bootstrap.rs,shutdown.rs,daemon.rs}
- apps/tdrive-cli/src/{main.rs,parse.rs,output.rs,commands.rs}
- crates/tdrive-api/src/{router.rs,auth.rs,onboarding.rs,files.rs,folders.rs,uploads.rs,media.rs,mount.rs,error.rs,events.rs,ipc.rs}
- crates/tdrive-service/src/{engine.rs,operation_queue.rs,events.rs}
- tests/compatibility/{cli_snapshots.rs,daemon_v6.rs,http_contract.rs}
- tests/e2e/{web_onboarding.rs,web_upload.rs,cli_daemon_upgrade.rs,auth_security.rs}

## 실제 HTTP routes: 현재 Go web.go에서 확인

- **UI**: /
- **파일**: /api/list, /api/find, /api/mkdir, /api/rm, /api/mv, /api/cp, /file
- **상태**: /api/status
- **온보딩**: /api/onboarding/status, /setup, /login, /code, /password, /cancel, /drives, /drive (모두 /api/onboarding 접두)
- **보안 설정**: /api/settings/token, /api/settings/proxy
- **미디어/썸네일**: /api/proxy/transcode, /api/proxy/status, /proxy-hls, /api/thumb
- **업로드**: /api/upload, /api/upload/status

각 URI는 **method, query, body, cookie/header, error JSON, response shape, status code, timeout, stream**를 Go snapshot에서 확인하고 맞춘다. URL만 같은 것은 API 호환이 아니다.

## 구현 순서

1. **Daemon owner**: single process with process lock, SQLite service, Telegram sync, media, mount endpoints, graceful shutdown. Startup 순서 secrets/config→lock→DB/schema→Engine→listener→worker; failure 시 역순 cleanup.
2. **IPC v6**: request ID/version/command/payload, typed responses/event streams, safe error codes. 기존 Go CLI 연결 시험; 새 기능은 protocol negotiation 또는 version bump 명시. 버전 불일치는 조용히 fallback하지 않고 명확한 upgrade 안내.
3. **Local security**: Unix socket 0600 + owner-only dir, Windows named pipe per-user restricted ACL/SID; Linux systemd socket activation은 필요한 경우만 별도 ADR. 다른 UID가 admin request를 쓸 수 없음.
4. **Rust CLI**: 현재 help, command names, options, --drive-id, --json, commands --json, machine-readable error, progress stderr/result stdout, exit codes/TTY password-stdin semantics를 oracle snapshots로 맞춤.
5. **Daemon supervisor**: CLI 자동 시작, PID file stale check, ping readiness(파일 존재만으로 성공 판정 금지), shutdown timeout, restart, existing Go daemon과 conflict 감지. 포트/소켓 경합이 나면 abort.
6. **HTTP server**: 기존 Go routes를 1:1 parity 우선. 가능한 새로운 API v2는 별도 prefix·typed schema에서 추가. Request body limit, path normalization, rate limit, cancellation, timeouts, 206/416, streaming backpressure.
7. **Web auth**: 현행 web token cookie/query bootstrap, SameSite, HttpOnly 및 constant-time compare, host-local token/no-auth via tailnet의 차이를 보존. CSRF/Origin/CORS 안전성을 향상시키려면 ADR + regression + 안내.
8. **Onboarding**: API ID/hash, phone/code invalid retry/2FA, cancel+retry, personal drive create/select, login 전만 이용 가능한 routes 정책. 웹에서 인증 정보가 로그/URL query로 유출되지 않도록.
9. **Web file UX**: 기존 HTML/Svelte 유지, JSON snapshot 동등. upload worker queue max/ids, status polling, failed/cancel/partial, gallery preview, thumbnails와 download proxy.
10. **API security tests**: token rotate invalidates old cookies, missing/invalid session 401/403, path traversal blocked, no credentials in JSON, unauthorized remote access, untrusted filenames output escaped.
11. **Events**: daemon streaming frame, upload/download progress, drive_scan_progress, live_sync_*; backpressure, ordering, reconnection, resource cleanup, UI 상태 재동기화.
12. **Compatibility transport**: GUI는 local/remote endpoint 선택, 클라이언트는 server API version negotiation. 구 Go 웹·백업 클라이언트는 새 API에 대한 regression before cutover.
13. **Observability**: health/readiness/liveness 구분, DB locked/Telegram offline/authorized state 구분, structured logs redaction, correlation ID, no token or private caption in error.
14. **Performance**: Go 대비 warm request/large listing/search/upload status의 p50/p95, memory/RSS, idle CPU 비교.

## 테스트 행렬

| 기능 | 요구 검증 |
|---|---|
| v6 IPC | Go↔Rust 동일 commands/errors + 프로토콜 충돌 명시 |
| --json CLI | output shape/exit codes/redirected stdout/stderr |
| onboarding | invalid code 재시도, 2FA, cancel, session preservation |
| token security | cookie/query/rotate/auth denial, Tailnet-only no-auth |
| file routes | path/ID/size/int64/206/416/timeout/stream |
| upload queue | restart/partial/cancel/slow Telegram/429 |
| frontend E2E | 기존 4개의 Playwright suite 재사용 |
| backup agent | 구 HTTP client도 새 Rust API와 통신 가능 |
| disconnect | daemon restart 중 GUI·웹 safe stale state 처리 |

## 완료 조건

- [ ] 기존 HTTP/IPC/CLI snapshot 계약 전량 통과 또는 명시적으로 승인된 versioned 변경
- [ ] Go Web UI와 Go backup agent가 Rust API 상대로 E2E 통과
- [ ] auth, secret redaction, localhost/tailnet surface 보안 테스트
- [ ] daemon crash/restart/parallel CLI, socket permissions, graceful shutdown
- [ ] CPU/RSS/latency 기준선 기록
- [ ] 신규 Rust API가 Phase 12 GPUI와 Phase 13 Docker에 안정적으로 제공됨

**롤백**: 현재 웹과 CLI를 Go server에 재접속. 반드시 Rust daemon을 완전히 종료/락 해제하고 Go를 실행; 동일 /data 동시 실행 금지.
