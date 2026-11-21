# Phase 02 — Golden Fixtures와 Go↔Rust 차등 검증

**목표**: 현행 Go 구현을 oracle로 삼아 wire/DB/crypto/HTTP/IPC/backup/미디어의 기계적 호환성 게이트를 만든다.

## 변경 예정

- tests/fixtures/{wire,sqlite,crypto,telegram,media,api,backup,mount}/ 와 manifest(hash, Go commit SHA, generator version, secret scrub status).
- 신규 test-only Go fixture exporter, backend/projection/*_test.go, crypto/*_test.go, daemon/*_test.go 보강.
- Rust tests/compatibility/와 crate별 contract suites (Phase 03 이후 실행).
- 기존 웹 Playwright spec 4종을 API mock 기반 사용자 관찰 oracle로 유지.

## Fixture 구현 세부

1. **TDX1**: projection/types.go의 모든 타입 — f, mkdir, rename, move, rmdir, tomb, meta, encfg, part, manifest, fcommit, freplace, dcommit, relocate, trash, harddel, untrash. 정규 직렬화/파싱, malformed/unknown version, base64, Unicode, duplicate keys, 이름 암호화.
2. **Projection**: root parent "", d:/f: ID, monotonic msg_id, skipped/tombstone/missing parent, move cycle 거절, repeat apply 멱등성, tamper/reject log, revision/restore/multipart.
3. **DB**: schema v16 fresh + personal channel migration + legacy database + interrupted migration. WAL, indexes, encryption policy, gallery tables, writable ops, replay rebuild; 전부 복제본에서 검증.
4. **TDE1**: deterministic test RNG/salt/nonce로 50B header·64KiB chunk·16B tag·final chunk 확인. 0/1/65535/65536/65537B, 여분 바이트, 잘림, tag 변조, 잘못된 암호, filename envelope.
5. **Telegram adapter fake**: 로그인 코드 오입력/2FA, stable random_id, accepted-but-no-receipt, retry, FloodWait, access hash 갱신, partial stream, DC redirection, reconnection.
6. **HTTP/IPC**: ProtocolVersion 6 command/error/event, status 200/202/206/401/403/404/409/416, token rotation, onboarding, WebDAV, content-range, progress, JSON 64-bit IDs.
7. **Backup**: 검증 전 원본 삭제 금지, corrupted JSON journal 격리, timeout/restart/multi-file·재전송, 암호화 금고 lock 상태와 평문 backup의 원래 의미.
8. **UI**: 현행 Svelte+Playwright의 folder/gallery/onboarding/workspace 경로 및 키보드/드래그 드롭/보안 에러.

## 차등 테스트 원칙

~~~mermaid
flowchart LR
  INPUT[fixture] --> GO[Go oracle]
  INPUT --> RUST[Rust candidate]
  GO --> NORMAL[normalized state]
  RUST --> NORMAL
  NORMAL --> DIFF[bytes / rows / events comparison]
  DIFF --> GATE{P0 discrepancy?}
  GATE -->|yes| BLOCK[block rollout]
  GATE -->|no| PASS[allow next phase]
~~~

- **byte equal**: 복원된 파일, wire canonical header, content hash, range.
- **DB state equal**: PK 정렬된 row, default/null, revisions, watermark, ignored/rejected op. timestamps/nonce는 고정 입력 또는 의미 기반 비교.
- **failure equal**: 잠긴 vault, 잘못된 암호, 불확실한 encryption policy, 조작 메시지는 예외 없이 fail closed.
- **idempotency**: 같은 op 2회, timeout 뒤 retry, 재부팅 시 중복 결과 없음.
- **observability**: event order, error code, 요청 ID; secret 데이터 노출 없음.

## 완료 조건

- [ ] 모든 위험 계약에 Go oracle 입력/예상값 준비
- [ ] negative/malformed/Unicode/크기 경계 및 crash tests 포함
- [ ] fixture manifest와 파일 SHA 검증 가능
- [ ] Rust adapter가 나중에 동일 fixture를 읽는 harness 확보
- [ ] Go CI 실제 실행 결과와 변경 SHA 기록
- [ ] P0 mismatch가 있으면 쓰기 기능 및 다음 Phase rollout 불허

**롤백**: 테스트 전용 추가 파일만 제거 가능. 실계정·개인 채팅·API secret이 fixture에 섞이면 산출물 사용을 중지하고 비밀 취급 사고로 관리.
