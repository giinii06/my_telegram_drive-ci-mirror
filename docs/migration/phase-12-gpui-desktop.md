# Phase 12 — GPUI 네이티브 데스크톱 UI

**선행** Phase 01/03 GPUI spike, Phase 11 API parity / **결과** macOS·Linux에서 Rust GPUI 앱이 기존 웹 TDrive UX 핵심 동작을 제공하며 서버와 완전히 분리되어 실행.

## 제품 정의 / 주요 제약

- GPUI는 **Rust 네이티브 GPU UI**다. WebView/Bun/Svelte의 또 다른 번들러가 아니며, Docker 서버 내부에서 표시하는 방식으로 설계하지 않는다.
- 기존 웹은 유지한다. GUI를 설치하지 않아도 Linux 서버·모바일 브라우저·백업 에이전트는 계속 기능.
- 우선 지원 목표: macOS Apple Silicon/Intel, Linux Wayland/X11(지원 upstream commit 기준). Windows는 실제 해당 GPUI 버전의 build/UI smoke가 통과할 때만 지원 선언. Android GPUI는 범위 밖.
- 현재 main의 cmd/tdrive/webui/index.html 및 tests/{drive,gallery,onboarding,workspace}.spec.mjs를 시각/동작 참조로 사용. 기존 HTTP API route는 Phase 11과 연결.

## 신규 Rust 모듈 구조 (가정; API 조사 뒤 수정)

~~~text
apps/tdrive-desktop/
  src/main.rs           # GPUI app initialize, theme, window
  src/app.rs            # top-level UI states, window routing
  src/api_client.rs     # authenticated HTTP/local daemon client
  src/events.rs         # server events and reconnect
  src/store.rs          # normalized drive/file cache
  src/router.rs         # onboarding, explorer, gallery, settings
  src/components/
    sidebar.rs
    breadcrumb.rs
    toolbar.rs
    file_table.rs
    file_card.rs
    transfer_queue.rs
    progress.rs
    dialog.rs
    toast.rs
    thumbnail.rs
  src/views/
    onboarding.rs
    explorer.rs
    gallery.rs
    preview.rs
    search.rs
    settings.rs
    transfers.rs
    vault.rs
  src/platform/
    file_picker.rs
    notifications.rs
    open_external.rs
  tests/
    app_state.rs
    view_models.rs
~~~

GPUI API/event/context/element의 실제 타입은 공식 Zed GPUI의 **pin한 커밋에 대한 spike**로 확정. 예시 경로는 제안이며 upstream이 안정적이라는 보증이 아니다.

## 화면/컴포넌트 구현 순서

1. **UI 기능 감사**: 현행 Svelte UI에서 화면, 상태, DOM interaction, keyboard, context menu, selection, drag/drop, preview, upload queue, gallery, file operations를 모두 inventory.
2. **Design system**: typography, light/dark, spacing, responsive window layout, sidebar, breadcrumb, status indicator, empty/loading/error state. 폰트·GPU 렌더링·한글 IME·접근성 키보드 포커스 확인.
3. **Connection state machine**: Disconnected → Connecting → Authenticating → Ready → Reconnecting/Unauthorized/Offline. 화면 재시작 시 API status로 canonical snapshot 다시 fetch.
4. **Onboarding**: API ID/hash entry, phone, code 재입력, 2FA(별도 secret field), personal drive select/create, cancel and expiry. 민감 데이터 GPUI state에서 필요 이상 복제하지 않음.
5. **Explorer**: pinned drive selector, 좌측 폴더 트리, breadcrumb, list/grid 보기, virtualized 10k rows, selected multiple, stable sorting, filter, search, rename/move/mkdir/trash/restore.
6. **File types**: 이미지/동영상/오디오/문서 인식, 앱이 지원하지 않는 형식의 외부 앱 열기, 파일명 충돌과 권한 에러 UI.
7. **Transfer UI**: bulk upload, drag/drop, per-file progress/wait/retry/cancel, stream API, error/no network, overwritten conflicts. 표시 progress는 서버에서 authoritative.
8. **Media/preview**: Phase 09의 tokenized local/remote stream 사용, image/thumbnail/HLS audio/video seek, 비지원 codec 안내, vault locked state. 미디어 복호화를 UI가 임의 구현하지 않음.
9. **Vault/permissions**: unlock/lock state, clipboard 정책, key handling은 daemon-side; 암호 입력은 secure controls, lock 후 preview/token/cache 즉시 비활성화.
10. **Mount UI**: mounted/attaching/stopped/draining/failed 상태 및 read-only/writable 구분, mount start/stop conflict, host OS permissions 안내. Docker 서버에는 mount 버튼 기능 차이를 명확히.
11. **Settings**: daemon endpoint local/remote, Tailscale offline handling, token rotation/session revocation, cache limits, logs(export sanitized), theme/locale/notifications.
12. **Async rendering discipline**: GPUI main render thread에서 network/crypto/DB/blocking decode 금지, background tasks + versioned immutable view-model + cancellation; stale response가 newer view state를 덮지 않도록 generation counter.
13. **Accessibility/locale**: 한국어/영어 UX, UTF-8/IME, VoiceOver/AT-SPI 가능성 검사, keyboard-only path, focus trap, contrast, reduced-motion (지원 범위 기록).
14. **OS lifecycle**: macOS app bundle/permissions, Linux .desktop/sandboxed secrets, tray/menu bar 가능성, close-to-tray vs stop-daemon 정책, multiple windows/tab state.
15. **Telemetry privacy**: 사용자의 파일명/채팅내용/휴대폰 번호를 desktop analytics/console/log에 전송하지 않음.

## 화면 연결 및 이벤트

~~~mermaid
sequenceDiagram
  participant UI as GPUI desktop
  participant API as Rust API
  participant CORE as Rust Core
  UI->>API: GET status/files (auth)
  API->>CORE: Read current projection
  CORE-->>API: file DTOs
  API-->>UI: snapshot + version
  UI->>API: upload stream / command
  API->>CORE: enqueue operation
  CORE-->>API: progress + final state
  API-->>UI: events
  Note over UI: render latest version; cancel stale requests
~~~

## UI 품질 Gate

| 테스트 | 검증 |
|---|---|
| Fresh install | cold start, onboarding, local server spawn/attach |
| Existing account | no forced data reset, available files visible |
| 10k file entries | virtualization/scroll/selection memory budget |
| multi-selection | keyboard, mouse, drag/drop, context menus |
| upload offline/retry | 진행률과 실제 remote state가 일치 |
| vault lock during preview | content/session access revoke |
| Tailscale reconnect | remote error handling + state refresh |
| Korean IME | 파일명 입력과 검색, Unicode normalization |
| desktop close/restart | server/daemon ownership and queue resume |
| unsupported platform | graceful web fallback, no false release claim |
| native UI smoke | screenshots/keyboard actions from host macOS/Linux CI |

## 완료 조건

- [ ] pinned GPUI upstream SHA와 MSRV, 라이선스, build targets 기록
- [ ] macOS native + Linux desktop window, keyboard, drag/drop, file dialog smoke
- [ ] Explorer/onboarding/gallery/transfers/vault/settings/mount parity check
- [ ] UI state tests, accessibility, localization and 10k entries performance
- [ ] 서버 headless CI/Docker build와 GPUI build 완전히 분리
- [ ] Windows/Android 기능 matrix와 fallback 명확

**롤백**: GPUI 앱만 비활성화/이전 버전으로 복귀. Rust server와 기존 Web UI/CLI 계속 유지. 앱이 DB 파일을 독자적으로 열거나 파괴하지 않기 때문에 UI 교체가 DB migration을 발생시키지 않는다.
