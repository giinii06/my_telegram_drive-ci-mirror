# Phase 05 — Telegram user-account MTProto와 인증 세션 교체

**선행** Phase 02~04 / **결과** Rust TelegramPort가 현행 gotd의 필요한 의미를 재현. 실제 원격 쓰기는 feature flag와 격리 계정 검증 후에만.

## 현재 구현 앵커

- backend/tgclient/client.go: Client/IdempotentSender 인터페이스, HistoryMessage, InputPeer, FloodWaitError, SendFileResult.
- backend/tgclient/{gotd.go,gotd_pool.go,gotd_range.go,download.go,upload_part_retry.go,write_coordinator.go,idempotency.go,floodwait_retry.go,getfile_limiter.go}
- backend/auth/{auth.go,session_storage.go,private_file.go,config.go,logout.go,group.go}
- backend/services/{auth,channel,personaldrive,user}/*
- backend/sync/{difference.go,sync.go}: history/pts/bounds.
- backend/core/engine.go: default gotd client와 liveActivity handler.

## 선택 전 spike: ADR-003

동일한 sandbox 계정에 대해 검증하되 **gotd 프로세스와 Rust를 동시에 연결하거나 계정 session file을 공유하지 않는다**.

1. **요구 표면**: 전화번호, 코드 재시도, 잘못된/만료된 코드 구분, 2FA hint/password, 로그인 전 상태, 기존 채널 목록/생성/초대/가입 신청/탈퇴, self ID, 로그아웃.
2. **읽기**: history pagination (min_id/offset_id), deleted/service placeholder, channel difference pts, reaction/notification와 무관한 실제 파일 메시지, direct user account access.
3. **쓰기**: SendControl, SendFile, multipart parts, random_id, progress, success-but-lost-receipt, stale access_hash retry, FloodWait.
4. **bytes**: getFile range, offset/limit/partial read, DC migration, CDN redirect, file ref expiration, media pool per DC, cancellation, 2GB 이상/ Telegram 현재 파일 한도.
5. **운영**: single long-lived connection, reconnect, backoff/jitter, bounded rate limit, close while requests running, arm64+amd64/native build, memory/CPU.
6. **선택 결과**: Rust-native vs TDLib FFI의 coverage 점수, binary size, license/MSRV, test coverage, release health, crash semantics. 후보 기능이 없으면 wrapper adapter 구현 범위/일정을 별도 문서화.

## 신규 코드 경계

- crates/tdrive-telegram/src/{port.rs,types.rs,client.rs,auth.rs,session.rs,history.rs,uploads.rs,downloads.rs,retry.rs,dc_pool.rs,errors.rs}
- crates/tdrive-config/src/secrets.rs (session material storage)
- tests/compatibility/telegram_contract.rs, tests/e2e/telegram_sandbox.rs
- raw TL/TDLib 타입은 내부에서만 사용하고 tdrive-types에는 배포하지 않는다.

## 상세 구현 순서

1. **Port + Fake**: Go Client의 모든 사용처를 call graph로 정리하고 필요한 async trait만 선언. Abort/cancel, bytes limit, unknown result를 typed로.
2. **Connection owner**: 하나의 authenticated connection, lazy start, self ID cache, background update loop, per-DC media pools, close/drain cleanup. 인스턴스 공유와 파편화 방지.
3. **로그인 상태기계**: Unconfigured → AwaitPhone → AwaitCode → CodeRejected(retry) → AwaitPassword → Authorized/Expired/RateLimited; 확인된 사용자 요청에 의해서만 인증정보 갱신. 2FA 비밀번호는 로그/에러/telemetry에 금지.
4. **Session serialization**: gotd 파일이 타 라이브러리에서 안전하게 import되는지 fixture로 증명. 안 되면 기존 파일을 백업한 상태로 **명시적 재로그인**; 무단 format conversion, 세션 제거 금지.
5. **Telegram channel mapping**: channel_id/access_hash/username/invite link, personal channel vs active browsed drive, pending join, unknown/private channel errors, ID int64.
6. **History API**: stable cursor and ordering, placeholder, access hash refresh on stale errors, difference pts, new message/media handling. 페이징이 진전되지 않으면 무한 루프 차단.
7. **Write semantics**: op 또는 파일마다 persistent stable random_id를 생성/재사용. 서버 수락 후 응답 유실 시 random_id를 바꾸지 않고 제한된 재시도. 결과 불명확 시 ErrSendOutcomeUnknown으로 worker에게 전달.
8. **FloodWait**: 수초/수분 rate limit을 정확히 해석하고 사용자 진행 이벤트(wait_seconds)로 전달. 전체 앱을 blocking sleep시키지 않음; deadline/cancel respect.
9. **Streaming IO**: chunked upload/download, progress, per-DC concurrency, backpressure, temp cleanup, byte-range remote errors, file_hash/size; large file 전체 메모리 로딩 금지.
10. **Live sync updates**: raw update를 channel dirty signal로 변환, 실제 state canonical source는 Phase 06의 history pull. handler에서 DB에 직접 apply 금지.
11. **Logout**: soft/full의 Go 정책(세션만 제거 vs configuration/database/cache 등) 보존; 사용자가 고른 source directory는 삭제 금지.
12. **Observability**: login token/phone/API hash/session, message caption, channel titles 등 민감정보 redaction. event name 및 safe error code 매핑.

## 핵심 실패 시나리오

| 상황 | 기대 |
|---|---|
| 잘못된 login code | 같은 phone_code_hash로 다시 입력 가능 |
| 만료된 code/2FA 실패 | 명확한 terminal/retry state, secret 보존 |
| timeout after accepted write | 동일 random_id로 중복 생성 차단 |
| lost network / DC redirect | bounded retry, correct source bytes |
| FloodWait 60s | UI에 대기 이유/남은 시간, cancel 가능 |
| access hash rotation | 한 번 갱신 후 동일 ID 재시도 |
| close during stream | 정상 cancellation + handle cleanup |
| history gap/placeholder | 프로젝트 sync가 재확인하도록 이벤트만 발행 |

## 완료 조건

- [ ] ADR-003 최종 결정과 threat/build/license 근거 첨부
- [ ] user auth/2FA, 채널 작업, history/difference, streaming, FloodWait, stable-ID 동작 등가
- [ ] 일부 실제 Telegram 테스트는 격리 계정과 명시적 동작 flag
- [ ] ARM64/amd64 compile + runtime smoke, crash/reconnect tests
- [ ] 세션 파일 이관 또는 재로그인 UX 방침·실제 결과 확인
- [ ] 원격 데이터 수정은 Phase 06 writer 검증 전 default-off

**롤백**: Rust Telegram flag off → Go backend를 안전하게 재시작. 로그아웃/세션 삭제는 자동 실행하지 않는다. 불명확 write 결과는 무조건 새로운 remote op로 재실행하지 않는다.
