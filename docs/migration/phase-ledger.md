# Rust 전체 이행: Phase 실행 원장 (full-parity execution ledger)

기준: blond11qq/my_telegram_drive, Go main 7edfea5456f4e9b18305e15e9de5d149976331da, Rust 누적 HEAD 89fbdd19188b4cd9213094ba1d8ab8039b63d92c (2026-11-22 작성).

이 문서는 기존 phase-00..15 설계 문서를 보강하는 **실행 기준**이다. Phase 번호나 CI green만으로 구현 완료를 선언하지 않는다. 각 하위 Phase (예: 04.b)는 **개별 코드/테스트 커밋과 정확히 일치하는 HEAD SHA의 GitLab CI**를 요구한다. 비밀/실계정/운영 데이터로 검증하지 않으며, 실제 Telegram 연동 테스트는 격리 test account에서만 진행한다.

## 정의와 진행 규칙

- **Not started**: 코드와 검증이 없다. **Partial**: 구체적인 테스트 가능한 작은 구현만 있다. **Verified increment**: 그 증분의 정확한 커밋 SHA에 대응하는 CI가 실제 통과했다. **Complete**: 모든 하위 작업, Go-Rust 계약·통합·실환경/플랫폼별 안전 게이트가 통과했다.
- MR은 이전 단계 브랜치에 쌓는 Draft/stacked MR을 기본으로 한다. 리뷰·안전승인 전 main merge, Go 삭제, 사용자 데이터/운영 계정/실 배포 변경은 금지.
- 각 코딩 하위 Phase마다 Go 근거/테스트 oracle → Rust 구현+regression → 독립 commit → HEAD CI(GitLab job별 통과) → 실패 job trace 근거 수정 commit → Draft MR 증거 기재. CI runner 미가동은 pass가 아니다.
- 04~09에 이미 있는 Rust 코드는 각각 **일부만** 구현됨. 04.a~09.a로 분류하며 기존 Phase 04~09를 완료 처리하지 않는다.
- 중대한 차이: 세션/MTProto는 사용자 계정 gotd 방식이지 Telegram Bot API가 아니다. GPUI는 호스트 네이티브 데스크톱 전용, Docker에는 GUI가 없는 Rust 서버만 제공한다. Android/WebView 및 Windows 지원은 플랫폼 실증과 별도 승인 전에는 폐기하지 않는다.
- **읽기/쓰기 완성은 별도**. 읽기 검증만 존재하는 crate가 라이브 writer/API 역할을 한다고 주장하지 않는다. 데이터 삭제, 암호화 쓰기, mount write는 end-to-end 검증 전 feature flag off.

## 세부 구현 Phase / 작업 단위

각 항목은 실질적인 하나 이상의 분리된 커밋 단위다. 기존 상세 명세는 docs/migration/phase-NN-*.md 참조.

### 00 — Go 동작 기준선
- [ ] 00.a 전체 Go 앱 CLI, daemon, web, backup, installer, Android 행동 inventory 및 실제 기준 SHA 고정
- [ ] 00.b Go full test/vet/build + Bun browser/Playwright + 플랫폼별 출력을 CI에서 검증
- [ ] 00.c 민감정보 없는 HTTP/IPC/DB/TDX1/TDE1/미디어 동작 스냅샷 및 오류 코드 채집
- [ ] 00.d 실제 대규모 파일/채널 기준 성능·메모리·재시작/복원 baseline

### 01 — Rust 설계·경계·의사결정
- [ ] 01.a workspace 및 한 프로세스 단일 writer 원칙 확정, import 계층 검사
- [ ] 01.b user-account MTProto Rust stack, session 지속성·로그인 방법 비교 spike 및 ADR
- [ ] 01.c GPUI의 macOS/Linux/Windows 버전별 지원과 event loop/IPC 설계 spike
- [ ] 01.d 공유 DTO, 오류·재시도, int64 JSON, cancellation/backpressure, 구성 루트 확정
- [ ] 01.e 출시 지원 매트릭스에 Android/Windows fallback 및 rollback·upgrade 기록

### 02 — Go↔Rust 검증 오라클
- [x] 02.a TDX1 parser 43-case curated fixture 증분 (완전한 모든 op parity는 아님)
- [ ] 02.b 각 17 operation, rendition/invalid UTF-8/중복 키/Unicode의 값과 header byte golden
- [ ] 02.c schema/migration/replay_log/dirents/revision/backup journal DB diff
- [ ] 02.d MTProto timeout/unknown-result/retry/reconnect/FLOOD_WAIT deterministic fake
- [ ] 02.e TDE1+vault KDF/key wrap/name encryption + media bytes + HTTP auth/API/IPC/CLI UI oracle
- [ ] 02.f e2e fault injection, offline, cancel, crash and P0 zero divergence gate

### 03 — Rust 공통 기반/CI
- [x] 03.a Cargo workspace, Rust inspect CLI, Rust/Go/Bun GitLab CI 일부 구축
- [ ] 03.b tdrive-config/tdrive-errors/tdrive-service crates, robust error redaction and tracing
- [ ] 03.c Tokio task owner, cancellation, queue/resource budgets, graceful shutdown
- [ ] 03.d ARM64/AMD64/macOS CI jobs, dependency license/security scan and locked supply chain
- [ ] 03.e integration fake Telegram+clock+FS, per-phase fault verification tooling

### 04 — Storage/config/session (현재 read-only DB inspector만)
- [x] 04.a read-only Go schema v16 integrity/table count/snapshot checker (검증 증분)
- [x] 04.b 실제 Go ListFolderContents의 folder/file SELECT, 정렬, tombstone/channel 격리 차등 검증 — 검증된 **증분**: [MR !10 / Pipeline #3098026998](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098026998)
- [ ] 04.c full schema and schema version migration fixtures (all supported legacy versions)
- [ ] 04.d read-only all core tables and normalized record digest, populated encrypted/multipart/trash snapshots
- [ ] 04.e secure config/session data paths, 0700/0600, symlink/reparse protections, corrupt/missing handling
- [ ] 04.f process/OS advisory+SQLite single writer lock preventing cross-Go/Rust collision
- [ ] 04.g consistent WAL-aware backup with hash manifest, restore and dry-run clone migration
- [ ] 04.h transactional typed SQLite writer + rollback, corruption, interrupted migration, foreign-key tests

### Phase 04 검증 증분 기록 (전체 Phase 미완료)

- **04.b folder listing:** [MR !10](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/10), HEAD `e5dc61165726de315cc4622301fdd8cd3662be84` — GitLab #3098026998 **success**.
- **04.c-a replay reader (보조 증분):** [MR !11](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/11), HEAD `ba398b96d7f1db85b8a4f8d8a7ff3a6624c346be` — GitLab #3098033516 **success**. 위의 04.c **legacy schema migration 전부 완료 아님**.
- **04.d-a core-table digest:** [MR !13](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/13), HEAD `c00a2a71054e2f5103b7575907ce039f02d635cc` — GitLab #3098129784 **success**. 다양한 실제 DB/암호화·multipart/휴지통 변형 커버리지 부족.
- **04.e-a private config:** [MR !16](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/16) — Unix 원자적 설정·세션 저장과 안전 경로 검사 구현, Windows ACL 및 동시 수정 race 등 미해결. CI 결과 확인 전 검증 선언 금지.
- **04.f-a backend lock:** [MR !17](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/17) — Go 호환 O_EXCL PID lock 증분. 컨테이너 PID namespace/공유 advisory lock 미해결.
- **04.g-a SQLite backup/restore clone:** [MR !18](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/18) — SQLite online backup, content digest, 매니페스트 검증, 복원용 clone 구현. CI 확인 전 부분 구현이며 프로덕션 복구는 미구현.
- [병렬 GitHub workflow](github-parallel-ci.md)는 [MR !15](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/15); GitHub 미러 동일 SHA의 실제 실행은 별도로 확인해야 한다.

### 05 — Telegram user-account transport (현재 network 미구현)
- [x] 05.a trait/port models and stable SHA-256 random_id Go golden vectors
- [ ] 05.b native MTProto library selection pinned; Telegram auth key storage and DC handling
- [ ] 05.c phone/code/2FA/onboarding/logout, session recovery without secrets in logs
- [ ] 05.d channel discovery/access hash, PTS, history pages, differences, per-channel state
- [ ] 05.e small/large streaming upload and remote downloads, multipart refs, full byte equality
- [ ] 05.f FLOOD_WAIT, reconnect, file reference refresh, stable random_id uncertain result resolution
- [ ] 05.g test account canary/isolated network full recovery and rate limit tests

### 06 — Projection/history/live sync (현재 제한된 parser/formatter)
- [x] 06.a TDX1 version/ops parser and 7 simple operation Go parity formatter 증분
- [ ] 06.b 17 operation encoding + decoding, rendition/unknown/future versions/Unicode compatibility
- [ ] 06.c canonical Go-compatible apply 17 ops and immutable replay_log in same SQLite transaction
- [ ] 06.d idempotent event dedupe, tamper, reject logs, revision/dirent/conflict/cycle handling
- [ ] 06.e initial history scan and bounded spool, watermarks, replay rebuild vs Go normalized DB diff
- [ ] 06.f live delta sync, reconnect, disordered or missing updates, backfill, stable channel order
- [ ] 06.g 10k/100k synthetic operations performance/crash/retry and convergence

### 07 — TDE1 vault, key and file-name crypto (현재 제한된 read-only decrypt)
- [x] 07.a TDE1 inspector and bounded authenticated read-only Go ciphertext decrypt fixtures
- [ ] 07.b streaming TDE1 encrypt/decrypt w/ 64KiB frames, final chunk/size/HKDF nonce parity
- [ ] 07.c Argon2id vault init/unlock/change password/wrapping, key material zeroization
- [ ] 07.d sealed filename v1, wrong-key/tamper behavior, migration compatibility
- [ ] 07.e vault lock generation, lease cancellation, concurrent media/mount/session access
- [ ] 07.f encrypted uploads/media/range/revisions/restore integration plus P0 security tests

### 08 — Files, upload, backup (현재 pure deletion-permit decision only)
- [x] 08.a safe backup deletion decision typed states + Go status goldens (no delete syscall)
- [ ] 08.b bounded upload queue, name/target normalization, multipart chunk upload
- [ ] 08.c file commit/replace/rename/move/copy/trash/restore/hard-delete with revision checks
- [ ] 08.d resumable download, content hash verification, cancellation and temporary file atomic rename
- [ ] 08.e uploader journal, uncertain Telegram write reconciliation, duplicate prevention
- [ ] 08.f Rust backup agent actual HTTP client, journal and remote verified source cleanup
- [ ] 08.g crash/restart/source-retention tests; never unlink before verified and durable journal

### 09 — Media/preview/transcode (현재 range decision only)
- [x] 09.a Go-like byte Range header parser (18 fixtures)
- [ ] 09.b Telegram-backed range reader, cancellation and strict full-content verification
- [ ] 09.c auth-scoped media sessions, token revocation, encrypted seek, content headers
- [ ] 09.d thumbnails/gallery extraction limits, caching and privacy policy
- [ ] 09.e MKV/MP4 remux, fMP4/HLS manifests and segment seek without undesired reencode
- [ ] 09.f client restart/corrupt encrypted chunks/206/416/HLS browser e2e and perf

### 10 — WebDAV/native mounts
- [ ] 10.a Go read-only WebDAV method/auth/path golden cases, isolated local capability
- [ ] 10.b authenticated read-only WebDAV GET/HEAD/PROPFIND and remote content lifecycle
- [ ] 10.c cache invalidation, negative cache, casefold/Unicode collisions, revision pinning
- [ ] 10.d durable local scratch journal, validated encrypted writable mount policy
- [ ] 10.e PUT/MOVE/COPY/DELETE atomic remote commit + crash recovery and conflict codes
- [ ] 10.f native macOS/Linux/Windows mount adapter support-gated smoke
- [ ] 10.g controller full state machine, detach/reconnect, Docker no-OS-mount behavior

### 11 — Rust daemon, IPC v6, HTTP, CLI, web
- [ ] 11.a tdrive-service production engine wiring, isolated owner and shutdown
- [ ] 11.b daemon v6 IPC framed protocol, unix peer permissions and Windows pipe ACL
- [ ] 11.c CLI all commands/options/JSON/error codes/daemon autostart parity
- [ ] 11.d HTTP all routes/methods/body/error/content JSON shape with tests
- [ ] 11.e auth cookie/token rotation/CSRF/loopback/Tailnet-only and onboarding user login
- [ ] 11.f async upload progress/events/websocket or legacy stream compatibility and queue limits
- [ ] 11.g existing web (Bun/Svelte/HTML) Go→Rust API integration, browser Playwright e2e
- [ ] 11.h Go backup client interoperability and media/permissions/timeouts fault tests

### 12 — Host-native GPUI desktop
- [ ] 12.a pinned supported GPUI platform matrix, minimal host-native window startup and CI smoke
- [ ] 12.b daemon discovery/auth and safe no-secret state/session UI
- [ ] 12.c onboarding/drive picker, tree/search, upload/download/progress/dnd GUI
- [ ] 12.d preview/media/gallery/vault controls/trash/revisions/mount/settings
- [ ] 12.e keyboard/IME/accessibility/1000+ rows/performance, error and offline UX
- [ ] 12.f macOS/Linux native app launch/quit/reconnect integration, Windows explicitly gated

### 13 — Rust headless Docker / Tailnet
- [ ] 13.a apps/tdrive-server headless binary with no GPUI linkage, Rust multi-stage Docker build
- [ ] 13.b existing /data/config/env compatibility + nonroot permissions and volume persistence
- [ ] 13.c Tailscale sidecar Tailnet-only secure Compose/health/liveness/readiness
- [ ] 13.d linux/arm64+amd64 builds, network/cold start/upload/upgrade/restart smoke
- [ ] 13.e volume backup/restore rollback rehearsal and prevent down -v in scripts

### 14 — Release/packaging and supply chain
- [ ] 14.a Rust Linux deb/systemd installer tests + legacy config and file retention
- [ ] 14.b Rust GPUI macOS app/dmg and CLI, signing status, Apple Silicon native UI smoke
- [ ] 14.c Windows Rust CLI/server/installer + GPUI supported matrix / explicit fallback
- [ ] 14.d Android legacy APK CI remains until dedicated Rust Android migration approval
- [ ] 14.e full GitLab/GitHub CI matrix (Go oracle, ARM64, AMD64, macOS, Windows, Docker, browser)
- [ ] 14.f supply chain pinning, SBOM/license/CVE/OCI reproducibility, checksum and rollback artifacts

### 15 — Canary, cutover and independent verification
- [ ] 15.a read-only snapshots and full DB + crypto + HTTP/CLI/IPC + content differential test
- [ ] 15.b isolated Telegram user account E2E for all ops/large media/vault/backup/mount
- [ ] 15.c Go shutdown, ownership lock, consistent DB snapshot, Rust dry run and restore rehearsal
- [ ] 15.d approved per-user/pilot cutover with conservative flags and observability thresholds
- [ ] 15.e crash, network loss, sidecar restart, flood, disk-full drills and exercised Go rollback
- [ ] 15.f all supported platforms and user-facing contracts complete, P0/P1 zero, security audit
- [ ] 15.g separate reviewed Go removal/legacy compatibility archival MR only after cutover approval

## 추적 가능성 / 의존성

순서: 00..03 evidence foundations → 04.storage + 05.telegram + 06.projection + 07.crypto → 08.files → 09.media + 10.mount → 11.service/api/cli → 12.desktop + 13.docker → 14.release → 15.cutover. 세부 구현은 다른 phase 일부를 선행하여 검증해도 전체 완료가 되지 않는다.

| 근거 코드 | 주 담당 Phase | 계약 테스트 |
|---|---|---|
| backend/projection/schema.go, read.go, log.go, writable_schema.go | 04 / 06 | SQLite v16 normalized rows, replay atomicy |
| backend/auth/*, backend/tgclient/*, backend/sync/*, backend/livesync/* | 04 / 05 / 06 | test account MTProto, session and replay |
| backend/crypto/* | 07 | Go-generated TDE1, vault, names |
| backend/services/*, cmd/tdrive-backup/* | 08 | remote file and source retention |
| backend/media/* | 09 | session, range, remux and HLS |
| backend/mount*/* | 10 | WebDAV and native mounted FS |
| backend/daemon/*, backend/core/*, cmd/tdrive/* | 11 | IPC v6, HTTP routes, CLI, web |
| packaging/*, Dockerfile, docker-compose.yml, .github/workflows/* | 12 / 13 / 14 | native and container launch |
| docs/migration/phase-15-cutover.md | 15 | e2e parity and no-loss rollback |

## 상태 증거 기록 형식

각 하위 Phase가 검증될 때 구현 문서 또는 이 원장에 다음을 기록:
- Phase ID, 목표, source Go symbol/file, dependency, acceptance test/snapshot
- branch, commit full SHA, MR target, exact matching CI pipeline URL, job statuses
- Go↔Rust meaningful output diff; security/error/crash cases; test environment platform
- remaining gaps. 일부 구현과 실제 서비스 전환 가능 상태를 절대 혼동하지 않기.
