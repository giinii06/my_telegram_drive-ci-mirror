# Phase 01 — Runtime/Crate 경계, ADR 및 기능 보존 범위

**목표**: 코드 교체 전 headless server, GPUI desktop, CLI/backup의 소유권·프로세스·데이터 흐름을 확정.

## 선행 조건/근거 파일

- Phase 00 기준선, architecture.md, compatibility-matrix.md.
- backend/core/engine.go, daemon/protocol.go, cmd/tdrive/web.go, docker-compose.yml, packaging/*.

## 세 실행 단위

1. **Rust server/daemon**: 단일 SQLite writer, Telegram session/sync, vault, media, 파일 작업, WebDAV, HTTP/IPC. Docker 또는 native host에서 headless 실행.
2. **Rust GPUI desktop**: 네이티브 윈도와 UI state/selection/preview/업로드/온보딩. DB나 MTProto를 직접 소유하지 않고 daemon API 사용.
3. **Rust CLI/backup**: IPC/HTTP client. --json 및 진행률 계약 유지. 백업 원본 삭제는 서버 업로드 검증 뒤에만.

현행 웹 UI는 독립 사용자 표면으로 유지하고 Phase 11에서 API 동등성을 증명한다. Android의 Go service + WebView는 별도 이행 결정 전 제거하지 않는다.

## 인터페이스 및 동시성 설계

- ChannelId/MessageId는 내부 i64 newtype. JS/JSON 경계에서 정수 정밀도 손실 방지 위해 decimal string.
- Telegram TL types, SQL rows, GPUI entities를 공용 DTO에 누출하지 않음.
- 요청별 cancellation, timeout, idempotency/correlation key, permission context, safe typed errors, structured tracing.
- 파일은 bounded stream/ReaderAt; 중간에 모든 바이트를 Vec로 모으지 않음.
- one data dir ↔ one daemon writer. 로컬 동시 기동/잠금/스키마 쓰기 차단; Go+Rust shared-data shadow 금지.
- vault lock 전후 generation gate, token revocation, pending read cancel; UI는 plaintext/key 캐시하지 않음.
- local loopback/remote Tailnet 각 API security model, CORS/Origin/CSRF/rate limit, token/no-auth transition.
- crash matrix: Telegram send succeeded + projection failed, DB transaction aborted, sudden shutdown, resume, stale session.

## 확정할 결정

| 주제 | 실증 과제 | ADR |
|---|---|---|
| Rust MTProto backend | login/2FA/history/range/floodwait/arm64 spike | 003 |
| Runtime/DB | WAL, shutdown, cancellation, transactions, throughput | 004/005 |
| GPUI | macOS/Linux window, UI controls, Windows gate | 010/011 |
| IPC | v6 frame compatibility 또는 versioned shim | 007 |
| OS mount | WebDAV 후 OS connectors 도입 가능성 | 014 |
| Secret lifecycle | vault.pass, log redaction, key handling | 009 |
| Android | 유지/fallback/독립 Kotlin-Rust 프로토타입 | 012 |
| CI/license | MSRV, pinned GPUI, cargo audit, runners | 015/018 |

## 검증 시나리오

- GPUI가 종료되어도 server 계속; 서버 중단 시 UI가 재연결 및 안전한 오류 표시.
- 다른 프로세스가 DB lock 보유 시 강제 takeover 금지.
- Telegram offline에서 metadata read 가능하나 불확실한 crypto policy에는 plaintext write 거절.
- Tailscale sidecar restarts / token rotation 동안 재인증·접속 오류 표준화.
- backup upload incomplete + network timeout에서 로컬 원본 보존.

## 완료 조건

- [ ] crate dependency DAG 및 단방향 adapter 관계 확정
- [ ] API/DB/secrets/telemetry 단일 owner 도식화
- [ ] 지원 플랫폼과 fallback 승인, 미지원은 명시적 blocker
- [ ] ADR에 근거·반례·성능·보안 점검 및 검토자 기록
- [ ] 수용 가능한 rollback과 feature flag 정책
- [ ] Phase 02에서 기계적으로 검증할 계약 목록 확정

**중단/롤백**: 기술 선택 미확정은 보류/스파이크로 남긴다. UI를 Docker에 넣거나 Bot API로 사용자 계정 기능을 대체하지 않는다.
