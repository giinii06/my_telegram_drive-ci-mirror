# TDrive Rust + GPUI + Docker 전환 마스터 플랜

- **기준 저장소**: blond11qq/my_telegram_drive
- **분석 기준**: main @ 7edfea5456f4e9b18305e15e9de5d149976331da (2026-11-21 확인)
- **문서 변경의 범위**: 계획/설계 문서만. 구현, DB 이관, 배포, 서비스 중단, 기능 삭제 없음.
- **최종 목표**: 공통 Rust 코어 + Rust 헤드리스 서버/CLI + 호스트에서 실행하는 GPUI 데스크톱 앱 + Linux Docker/Tailscale 운영. 현재 웹 사용자와 서버 사용자, 백업 사용자, 모바일 사용자의 기존 데이터를 잃지 않는다.

## 목표를 정확히 해석하기

GPUI는 네이티브 GPU 데스크톱 UI다. Docker 컨테이너는 **GPU 화면을 표시하는 GPUI 앱을 실행하는 용도**가 아니라 Rust 백엔드와 웹 API를 실행하는 용도다. GPUI를 브라우저 화면으로 간주하지 않는다. 현행 웹 UI는 마이그레이션 동안 유지하고 HTTP API가 완성된 다음 선택적으로 재설계한다. **기존 Android 앱을 Rust+GPUI로 즉시 교체한다는 의미가 아니다**. Android와 Windows GPUI 호환성은 별도 게이트로 판단한다.

~~~mermaid
flowchart LR
  TG[Telegram MTProto] <--> CORE[공통 Rust 코어]
  DB[(기존 TDrive SQLite)] <--> CORE
  CORE <--> DAEMON[Rust daemon / IPC]
  DAEMON <--> API[HTTP API + WebDAV + media]
  DAEMON <--> CLI[Rust CLI / backup agent]
  DAEMON <--> DESKTOP[GPUI native desktop]
  API <--> WEB[현행 웹 UI - 유지]
  API <--> TS[Tailscale sidecar / tailnet]
  subgraph Docker_on_Linux[Docker Compose - Linux headless]
    API
    TS
  end
~~~

그림은 논리적 관계다. 서버/데스크톱이 동일 PC에서 실행될 때는 하나의 프로세스/서비스 소유권 규칙을 지킨다. Docker 안에는 GPUI 창이 포함되지 않는다.

## 전역 불변조건

1. **원격 메시지 보존**: Telegram의 TDX1 제어 메시지, 파일 메시지, multipart 및 revision 체계를 재해석하거나 소급 수정하지 않는다. 새 프로토콜 도입은 버전·하위 호환성 검증 전 금지.
2. **암호화 보존**: TDE1 / 암호화 파일명 / Argon2id 키 래핑을 변경하지 않으며, 암호를 모르는 상태에서 데이터를 자동 재암호화하지 않는다.
3. **로컬 저장소 보존**: 기존 SQLite와 인증 파일을 백업 없이 새 바이너리로 직접 변경하지 않는다. 데이터 변환은 스냅샷·검증·복귀 절차가 있어야 한다.
4. **단일 작성자**: Go와 Rust가 같은 데이터 디렉터리/SQLite/Telegram 동작에 동시에 쓰지 않는다. 비교 실행은 데이터 디렉터리 복제본 또는 읽기 전용 fixture로만.
5. **행동 호환성**: CLI, HTTP, 스트리밍, 백업, 마운트의 기존 사용자 흐름을 호환성 매트릭스에 따라 보존한다. 사용자가 소유한 백업 원본은 검증 전 삭제 금지.
6. **안전한 실패**: 오프라인, FLOOD_WAIT, 세션 오류, 네트워크 timeout, vault 잠금, 손상 DB에서 fail-closed 및 재시도 경계를 유지.
7. **배포 격리**: 문서 MR은 기능 릴리스가 아니며, Phase마다 별도 MR, 증거, 검토자 승인, go/no-go 결정이 필요하다.

## Phase 별 문서

| Phase | 핵심 산출물 | 선행 조건 | 문서 |
|---|---|---|---|
| 00 | 현재 동작 기준선과 위험 식별 | 없음 | [00 baseline](phase-00-baseline.md) |
| 01 | ADR/모듈·프로세스 경계 확정 | 00 | [01 architecture](phase-01-architecture.md) |
| 02 | Go↔Rust golden fixture, differential harness | 00~01 | [02 contracts](phase-02-contract-tests.md) |
| 03 | Cargo workspace / 설정·관측·CI 골격 | 01~02 | [03 workspace](phase-03-rust-workspace.md) |
| 04 | SQLite·config·session 영속성 | 02~03 | [04 storage](phase-04-storage.md) |
| 05 | MTProto 사용자 로그인/세션/전송 어댑터 | 02~04 | [05 telegram](phase-05-telegram.md) |
| 06 | TDX1 projection / history sync / live sync | 04~05 | [06 projection](phase-06-projection-sync.md) |
| 07 | TDE1 vault·파일명 암호화·잠금 | 02·04·06 | [07 crypto](phase-07-crypto.md) |
| 08 | 파일 작업·업로드/복원·백업 에이전트 | 05~07 | [08 files](phase-08-files-backup.md) |
| 09 | range/preview/MP4/HLS 리먹싱 | 07~08 | [09 media](phase-09-media.md) |
| 10 | WebDAV/FUSE·마운트 수명주기 | 08~09 | [10 mount](phase-10-mount.md) |
| 11 | Rust daemon/CLI/HTTP/web 이행 | 06~10 | [11 API](phase-11-server-cli-web.md) |
| 12 | GPUI 데스크톱 UI | 11 | [12 GPUI](phase-12-gpui-desktop.md) |
| 13 | Rust Docker·Compose·Tailscale·업그레이드 | 11 | [13 Docker](phase-13-docker.md) |
| 14 | Linux/macOS/Windows 패키지와 GitLab/GitHub CI | 11~13 | [14 release](phase-14-packaging-ci.md) |
| 15 | shadow 검증·파일럿·전환·Go 코드 퇴역 판단 | 02~14 | [15 cutover](phase-15-cutover.md) |

**병렬화**: 09와 10은 데이터 계약이 확정된 뒤 일부 병렬 가능, 12와 13은 11의 안정된 API 이후 병렬 가능. 14는 별도 빌드 matrix로 병렬화 가능. Phase는 반드시 **독립적인 소규모 MR** 단위로 분할한다.

## 각 후속 Phase MR의 필수 증거

- [ ] 기준 SHA와 변경 파일 / 해당 Go 원본 파일 매핑
- [ ] 영향 경로, 실패 모드, 보안·비밀 처리, 동시성 불변식
- [ ] 최소 단위 테스트 + Go↔Rust 계약/fixture 테스트 + 해당 플랫폼 통합 테스트
- [ ] CI job/commit SHA 및 실제 실행 결과. 빌드·테스트 수행 사실을 추정하지 않는다.
- [ ] migration dry run, 데이터 스냅샷 및 다운그레이드 경로가 필요한지 명시
- [ ] 실패 시 feature flag 비활성화 또는 롤백 절차, 관측 지표, 결정 기록 갱신

## 문서 사용법

Phase 00~02는 코드를 교체하기 전에 완수한다. 각 문서의 **완료 조건**을 통과할 때만 다음 Phase로 넘어간다. 위험 결정은 [decision-log](decision-log.md)에 ADR로 등록하고, 테스트/데이터의 **필드 단위 비교 기준**은 [compatibility-matrix](compatibility-matrix.md)를 기준으로 한다.
