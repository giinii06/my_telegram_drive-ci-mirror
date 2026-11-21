# ADR 결정 기록 / 오픈 질문

## 상태

**확정(제약)**: 사용자 요청 및 현행 코드에서 직접 도출되는 조건. **제안**: 구현 Phase에서 승인받을 기본안. **보류**: spike 없이는 결정 불가능.

| ADR | 상태 | 안건 | 현재 방향 | 확정 게이트 |
|---|---|---|---|---|
| 001 | 확정(제약) | GPUI와 Docker 경계 | GPU 데스크톱은 호스트, 서버는 headless Rust container | Phase 01 |
| 002 | 제안 | 웹 UI | 기존 Svelte/HTML 및 Bun E2E 유지 | Phase 11 |
| 003 | 보류 | Rust Telegram 엔진 | Rust MTProto native(grammers 계열 등) vs TDLib FFI; Bot API는 제외 | Phase 05 spike |
| 004 | 제안 | DB | SQLite 원형 보존; Rust driver는 rusqlite/sqlx 비교 | Phase 04 |
| 005 | 제안 | Runtime | Tokio 또는 동등 runtime, bounded task/stream/Cancel | Phase 03 |
| 006 | 제안 | API | 버전별 DTO, OpenAPI snapshot, stable safe error code | Phase 11 |
| 007 | 보류 | CLI↔daemon 전송 | v6 local IPC shim 또는 local authenticated HTTP | Phase 11 |
| 008 | 제안 | 데이터 변환 | snapshot → dry run → verify → atomic activation → rollback | Phase 04/15 |
| 009 | 보류 | vault.pass auto-unlock | 기존 동작 호환 vs opt-in vault secret | Phase 07/13 |
| 010 | 보류 | macOS GPUI | upstream pinned SHA, bundle/sign/notarization | Phase 12/14 |
| 011 | 보류 | Windows GPUI | version pin + native build/GUI E2E 전 확약 금지; web fallback | Phase 12/14 |
| 012 | 보류 | Android | 기존 Go/WebView 유지 vs 별도 Rust JNI/FFI 사업 | Phase 14/15 |
| 013 | 제안 | Docker base | multi-stage, reproducible digest, 최소 권한 nonroot | Phase 13 |
| 014 | 보류 | 마운트 | WebDAV parity 우선, FUSE/WinFsp는 OS별 판단 | Phase 10 |
| 015 | 제안 | CI | GitLab runners 확인 후 Rust+Go parity matrix | Phase 03/14 |
| 016 | 제안 | destructive action | purge/hard delete는 별도 flag+confirm+idempotency | Phase 06/08/15 |
| 017 | 보류 | tailnet 인증 | 현행 no-auth + network-only vs API-token 강화의 호환성 | Phase 11/13 |
| 018 | 제안 | 공급망 | crate MSRV/license/CVE, GPUI commit provenance 고정 | Phase 03/12/14 |

## Telegram 엔진(ADR-003) 증거 표

필요한 기능: 전화번호 user login, code 재입력, 2FA, 개인/공유 채널, history/pts/update, 파일 업로드, Telegram DC의 byte-range 다운로드, FLOOD_WAIT, stable random_id, CDN/access_hash 갱신, ARM64/AMD64, 세션 재시작.

| 선택지 | 장점 | 주요 리스크 |
|---|---|---|
| Rust native MTProto | 러스트 단일 빌드 가능 | 기능 갭, session 호환, maintainer/프로토콜 레이어 |
| TDLib + Rust binding | 풍부한 Telegram 기능 후보 | C++/FFI, 이미지 크기, 배포, cross-compile, shutdown |
| Telegram HTTP Bot API | 단순 | 현행 사용자 계정/채널 사용 사례를 대체하지 못하므로 **제외** |

최소 실험: 격리 계정 로그인/로그아웃, 역사 10만 메시지 pagination, 대형 파일, DC 전환, rate limit, reconnect, 2FA retry, ARM64 container. 각각 Go fake/real sandbox와 수치 및 데이터 바이트로 비교한다. **동일 계정 세션을 두 프로세스에서 동시에 사용하지 않는다.**

## GPUI(ADR-010/011) 사전 확인

- 공식 Zed GPUI의 정확한 git SHA / toolchain / features pin.
- macOS Apple Silicon + Linux Wayland/X11 window, input, file chooser, 이미지, virtual list, keyboard/accessibility, GPU fallback 검증.
- Windows 지원은 실제 upstream 버전의 빌드/실행 테스트가 있기 전 보증하지 않는다.
- 서버 중단 후 reconnect, logout, vault lock을 desktop에서 확인. GUI를 Docker로 실행해 호환성을 해결한다는 전제 금지.

## 데이터 및 보안 질문

1. 오래된 Go 클라이언트가 모르는 TDX op를 어떻게 처리하는가?
2. gotd session 이식을 변환기로 할지, 안전한 재로그인을 제공할지?
3. media token/vault key/cache가 locking/restart 후 무효화되는가?
4. backup 원본 삭제 전에 size 이외 hash 검증이 가능한가?
5. Docker /data의 UID·GID·권한·기존 경로 매핑은 어떻게 유지할까?
6. token-less tailnet UX에서 HTTP 인증을 강화할 때 기존 사용자와 충돌이 없는가?

각 ADR은 선택지/반례/위협 모델/owner/근거 SHA/결정일/후속 테스트/철회 조건을 기록해야 확정 가능.

참고: https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md , https://github.com/tdlib/td , https://github.com/Lonami/grammers .
