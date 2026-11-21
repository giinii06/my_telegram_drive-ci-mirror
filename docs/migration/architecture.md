# 목표 Rust 아키텍처와 프로세스 경계

## 1. 호스트/컨테이너 배치

**GPUI 앱**은 데스크톱 OS에서 실행하여 GPU와 창 관리자에 접근한다. **tdrive-server**는 GUI 의존성 없는 Rust 바이너리이며 Linux Docker 또는 호스트 프로세스로 실행한다. **tdrive-cli/tdrive-backup**은 daemon API 소비자다. 코드 공유는 crate를 통해, 원격 연결은 버전 명시된 API를 통해 한다. Android WebView/기존 웹 앱은 Rust API 전환 기간 유지.

~~~mermaid
flowchart TB
  subgraph Host[사용자 호스트 - macOS/Linux, Windows는 별도 지원 판정]
    GPUI[apps/tdrive-desktop GPUI]
    CLI[apps/tdrive-cli]
    BA[apps/tdrive-backup]
  end
  subgraph Server[Linux Docker 또는 host-native]
    API[apps/tdrive-server HTTP + WebDAV]
    IPC[local owner-only IPC]
    DOMAIN[tdrive-core / services]
    DB[(TDrive SQLite, single writer)]
    TG[tdrive-telegram]
    API --> DOMAIN
    IPC --> DOMAIN
    DOMAIN --> DB
    DOMAIN --> TG
  end
  GPUI <-->|API / events| API
  CLI <-->|IPC or API| API
  BA <-->|authenticated API| API
  TG <--> TELEGRAM[Telegram MTProto]
  BROWSER[기존 Web UI] <--> API
~~~

초기에는 desktop이 loopback HTTP를 사용하는 것도 허용하되 로컬 API의 loopback binding, bearer token, CSRF/Origin, CORS, URL capability를 보안 설계에 포함한다. 복잡한 IPC bridge를 성급히 구현하지 않는다.

## 2. 후보 워크스페이스 (계획; 실제 생성은 Phase 03)

~~~text
Cargo.toml / Cargo.lock / rust-toolchain.toml
crates/
  tdrive-types/       # IDs, path, errors, clocks, serializer
  tdrive-config/      # config dirs, migration, secret store boundaries
  tdrive-telegram/    # MTProto adapter trait and backend
  tdrive-storage/     # SQLite + transaction APIs
  tdrive-projection/  # TDX1 parser, canonical event application
  tdrive-sync/        # history replay + live sync and scheduling
  tdrive-crypto/      # vault / TDE1 / filename crypto
  tdrive-files/       # uploads, downloads, revision, backup contracts
  tdrive-media/       # ranges, sessions, remux and HLS
  tdrive-mount/       # WebDAV, adapters, optional OS mounts
  tdrive-service/     # application use-cases and Engine wiring
  tdrive-api/         # HTTP, local RPC, versioned DTOs
apps/
  tdrive-server/      # no GPUI dependency
  tdrive-cli/
  tdrive-backup/
  tdrive-desktop/     # GPUI, only desktop targets
tests/
  fixtures/           # sanitized Go-generated golden fixtures
  compatibility/      # Go/Rust differential tests
  e2e/
docker/                # maintained alongside old image during transition
~~~

각 crate는 inward dependency만. API/GPUI가 DB SQL을 호출하면 실패. Telegram 타입(원본 TL generated struct)이 domain/API로 누출되면 실패. DI는 trait-based ports, explicit resource ownership; Tokio runtime 수명/종료와 DB 단일 writer를 하나의 상위 composition root가 관리.

## 3. 핵심 port / DTO 설계 예

~~~rust
// 개념적 설계 예시이며 컴파일되는 코드가 아님.
trait TelegramPort {
    async fn get_history(&self, channel: ChannelId, cursor: Cursor) -> Result<Page>;
    async fn send_control(&self, channel: ChannelId, raw: &str, stable_id: i64) -> Result<MessageId>;
    async fn upload(&self, spec: UploadSpec, stable_id: i64) -> Result<RemoteFile>;
}
trait ProjectionStore {
    async fn apply(&self, channel: ChannelId, msg: MessageId, op: WireOp) -> Result<ApplyStatus>;
    async fn get_watermark(&self, channel: ChannelId) -> Result<MessageId>;
}
struct AppError { code: ErrorCode, safe_message: String, retryable: bool }
~~~

실제 시그니처는 futures/Send, 객체 안전성, cancellation, streaming ownership을 검증하고 문서 ADR을 갱신한다. 큰 파일은 Vec<u8> 일괄 읽기가 아니라 async stream / ReaderAt 계약으로 처리한다. channel_id와 msg_id는 서버 내부에서 64-bit 정수, JS UI 경계에서는 **문자열**로 전달하여 정밀도 손실을 막는다.

## 4. 데이터 소유권 및 동시성

- 한 데이터 디렉터리 = 단일 active writer. DB WAL, busy_timeout, fsync, checkpoint, 마이그레이션 lock은 설계에서 확정하고 smoke-test.
- 모든 쓰기: Telegram op에 stable random_id → 전송 결과 확인 → replay log + projection transaction. 결과 불명확하면 새 random_id를 생성하지 않는다.
- 원격 전송과 로컬 커밋 사이 crash는 정상적인 복구 대상. 무조건적인 원자성 주장은 금지.
- 세션 캐시·미디어 토큰·임시 파일·암호 키는 데몬 소유. GPUI는 복호 키를 렌더 상태에 남기지 않는다.
- 대기열과 병렬성: per-channel sync serialize; global/file upload limit; bounded queues, cancellation/shutdown drain.
- Live updates는 invalidation hint, canonical state는 history pull. 동일 메시지/중복 이벤트는 재생해도 결과가 같다.

## 5. 모듈별 디커플링 순서

1. **계약 고정**: Go parser/DB/crypto/API golden vectors.
2. **Storage/Projection 구현**: Telegram 없는 fake로 재현 가능.
3. **TelegramPort 구현**: 실제 계정은 격리 테스트 전용.
4. **Use-case 연결**: Go와 Rust 병렬 검증 but never same state write.
5. **서버, 웹, GPUI 연계**: DTO가 확정된 이후 독립 교체.
6. **Docker/패키지 전환**: 시스템 통합 테스트와 롤백 준비 후.

## 6. 비목표와 플랫폼 가드

- 1차 전환에서 Telegram Bot API로 이동, 원격 메시지 일괄 리라이트, 강제 계정 재가입, 기존 암호화 포맷 재설계는 **하지 않음**.
- Windows/macOS/Linux 패키징은 지원 검증에 따라 단계별. GPUI 공식 저장소가 제공하는 실제 버전별 지원 여부를 pin 검증한다. Windows 미지원 조합이면 기존 웹/호스트 CLI를 유지하고 blocker 문서화.
- Android는 현재 WebView/Go shared lib를 무작정 GPUI로 바꾸지 않는다. 별도 기술 평가/기능 유지 정책 확정.
- 보안상 Docker daemon에 privileged 권한을 기본 부여하지 않는다. Tailscale의 tun/NET_ADMIN은 분리된 사이드카에 필요한 최소 권한만 유지.

## 7. 참고 구현 문서

- GPUI upstream: https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
- Telegram 공식 TDLib: https://github.com/tdlib/td
- Rust native MTProto 후보에 대한 검증은 decision-log.md에 기록. crate 선택을 확정된 사실처럼 다루지 않는다.
