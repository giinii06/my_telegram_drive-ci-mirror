# Go → Rust 호환성 검증 매트릭스

이 표는 **목표**이지 통과 결과가 아니다. 테스트 생성 시 모든 샘플은 비밀 없는 fixture로 보관한다. 새 구현이 데이터 호환성을 통과하기 전 Go 서버를 제거하지 않는다.

| 계약 | 현재 Go 근거 | 필수 Go↔Rust 증거 | 실패 시 |
|---|---|---|---|
| TDX1 직렬화·파싱 | projection/wire.go, types.go | 모든 op 타입 roundtrip; unknown/malformed/ordering/base64/Unicode/UTF-8/duplicate keys 경계값 | Parser 사용 금지 |
| projection 결정성 | projection/apply.go, log.go | 같은 (channel,msg,op) sequence의 테이블 diff; root empty, d:/f: ID, duplicate/rejected ops, cycle, tamper | sync 금지 |
| schema v16 | projection/schema.go, writable_schema.go | 실 DB 스냅샷 on-disk read, migration, rollback; content/revision/gallery/dirent/enc 관련 count/hash | writer 금지 |
| history & live sync | sync/*, livesync/* | 역순/중복/삭제/placeholder, FLOOD_WAIT, stale access hash, reconnect, scan watermark | live sync 금지 |
| secret/session | auth/*, datadir/* | 경로/권한, 원자 저장, 오류 시 불변, logout soft/full, 세션 변환 실패 시 재로그인 경로 | 인증 배포 금지 |
| TDE1 | crypto/stream.go, random_access.go | header(50B), chunk(64KiB), final tag, nonce, HKDF, plaintext 크기, truncated/tampered stream, chunk boundary | 암호화 쓰기 금지 |
| vault | crypto/vault.go, name.go | Argon2id params, XChaCha key wrap/key check, 이름 v1 envelope, wrong password, lock after concurrent session | vault 구현 배포 금지 |
| upload/download | services/file/*, tgclient/* | stable random_id, unknown result, multipart manifest, interrupted retry, cancel cleanup, hash/size equality | file write 금지 |
| gallery/preview | media/*, projection/gallery* | range GET, multipart, content-type, ETag/206/416, image limits, thumbnails, HLS playlists seek/audio | media switch 금지 |
| mount | mount*/* | WebDAV methods, write atomicity, fs policy, path collision, Unicode, encrypted unlock, crash recovery | mount write 금지 |
| daemon IPC | daemon/protocol.go | v6 JSON frames, valid command/error, progress event, version mismatch, Unix ACL/Windows pipe | CLI 전환 금지 |
| web API | cmd/tdrive/web*.go | routes, fields, status codes, auth, onboarding, upload queue, file proxy, token rotation | 웹 전환 금지 |
| CLI | cmd/tdrive/* | command aliases, --json, stderr progress/stdout output, exit/error codes | native CLI 전환 금지 |
| backup | cmd/tdrive-backup/* | journal restart, corrupt journal quarantine, remote size/finish verify then local delete | 원본 삭제 기능 금지 |
| Docker | Dockerfile, docker-compose.yml, docker/entrypoint.sh | /data persistent, cold start/onboard, tailnet reachability, graceful stop, no mount, arm64/amd64 | image 전환 금지 |
| 설치본 | packaging/*, install.sh | existing config migration, start/stop, uninstall/no data erase, package restore | 릴리스 금지 |

## Golden fixture 데이터셋 최소 요건

- 각 TDX1 op 타입의 정상/비정상/구버전 메시지, Unicode 정규화·이상 이름, 중복 parent와 cycle, 대용량 multipart 및 파일 revision.
- 빈/가득 찬 채널, 암호화된 개인 드라이브, 잘못된 암호, 잠금/해제, 이름만 암호화된 케이스.
- 암호화 데이터 길이: 0B, 1B, 65535B, 65536B, 65537B, 2*65536B, 마지막 tag 잘림, 헤더 변조, 추가 바이트.
- Telegram 응답: 정상, timeout 뒤 성공, FLOOD_WAIT, 취소, hash 갱신, history skip, reconnect.
- SQLite: 새 설치, schema 16, 깨진 migration, 부분 업로드, projection rebuild, replay tamper, backup journal 손상.
- HTTP: JSON/error / 401/403/404/409/429/500, Range 206/416, token/no-auth 모드, SSE 또는 이벤트 프레임 순서.
- UI: 접근성, 키보드, 파일 drop, 1,000+ rows virtual scroll, 검색·필터·선택·업로드 취소.

## 값 비교 규칙

- byte-for-byte: TDE1 복호화 결과, TDX1 canonical header(동등 raw가 요구되는 경로), 파일 payload, checksum.
- 정규화 비교: DB row 정렬 후 실제 필수 컬럼, JSON 필드와 int64 문자열, 명시된 optional/null semantics.
- 상태기계 비교: FLOOD_WAIT/cancel/retry, local/remote 상태 수렴, event ordering.
- 사용자 관찰 기준: 기존 CLI 명령/옵션, API status/error, 웹 버튼 흐름, 복원된 파일 bytes.
- 비결정 값(timestamp/랜덤 nonce) 비교는 **정해진 clock/RNG/fake**로 고정하거나 decode 의미를 비교한다. 암호화 랜덤 nonce를 통째로 같다고 강제하지 않는다.

## Gate 정책

- P0: data loss, unauthorized plaintext, secret leak, duplicate destructive write, missing source delete guard **0건**.
- P1: 기능 오류 및 재시도 교착 **0건**(출시 대상 기능 기준).
- 다른 플랫폼 미지원은 명시적 feature matrix와 fallback이 있어야 한다.
- CI 미실행/러너 없음은 '통과'가 아니라 '검증 불가'로 보고한다.
