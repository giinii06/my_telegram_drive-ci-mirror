# Rust migration implementation status — updated 2026-11-22

현재 branch lineage: docs plan branch → Phase 04 → 05 → 06 → 07 → 08 → 09 → full parity plan. 원래 Go main은 수정/배포되지 않았으며 Rust 코드는 운영 서버에 연결되지 않았다.

**이 표의 "CI verified"는 해당 증분의 테스트만 통과했다는 뜻이고 phase 전체 완료가 아니다.** 모든 미구현 작업의 상세 목록은 [phase-ledger](phase-ledger.md)를 보라.

| Phase | 실제 Rust 증분 | GitLab 증거 | 전체 Phase 완료 |
|---|---|---|---|
| 00~03 | Go contract guard, curated TDX1 parser fixtures, Rust workspace + inspect CLI | inherited Phase 04 CI | 아니오 |
| 04.a | Go 생성 SQLite v16 snapshot read-only schema/integrity/row counts | b4733e9 / pipeline 3097711065 | 아니오 |
| 04.b | detached Go-compatible 폴더·파일 목록, tombstone/Unicode/channel oracle | e5dc611 / [3098026998](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098026998) 성공 | 아니오 |
| 04.c-a 보조 | bounded replay_log page, 원본 header SHA256, 채널별 커서 | ba398b9 / [3098033516](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098033516) 성공 | 아니오 |
| 04.d-a | 14 Go v16 core table row data digest 비교 | c00a2a7 / [3098129784](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098129784) 성공 | 아니오 |
| 04.e-a | Unix private config/session 경로·권한·원자적 저장 | [MR !16](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/16), CI 검증 대기 | 아니오 |
| 04.f-a | Go/Rust backend.lock exclusive create (PID namespace + OS advisory 미완) | [MR !17](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/17), CI 검증 대기 | 아니오 |
| 04.g-a | WAL-aware backup, SHA256 manifest 검증, no-clobber offline restore clone | [MR !18](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/18), CI 검증 대기 | 아니오 |
| 05.a | offline MTProto trait model + Go parity stable random_id | 9c4031c / pipeline 3097721220 | 아니오 |
| 06.a | 7 simple TDX1 writer formats + 14 Go parity cases | 3c45de1 / pipeline 3097739847 | 아니오 |
| 07.a | 8 MiB 제한 authenticated TDE1 read-only decrypt + 6 Go cipher fixtures | 89243d8 / pipeline 3097788263 | 아니오 |
| 08.a | backup deletion-permit decision type-state; unlink 미구현 | dc6e0d7 / pipeline 3097799701 | 아니오 |
| 09.a | Go-equivalent Range parser, 18 fixtures; media server 미구현 | 89fbdd1 / pipeline 3097808960 | 아니오 |
| 10~15 | production Rust WebDAV/server/GPUI/Docker/release/cutover 미구현 | 검증 증거 없음 | 아니오 |

사용자가 의존할 수 없는 것: **Rust Telegram 로그인/전송, mutable replay projection writer, vault key lifecycle, 업로드·다운로드·운영 백업 에이전트, HTTP/WebDAV server, native GPUI GUI, Rust Docker image, Go→Rust production cutover 전부 미구현.** SQLite offline backup은 앱·운영 백업 에이전트 구현과 다르다.

### 다음 작업 순서

1. [전체 실행 원장](phase-ledger.md)의 04.e/f/g 구현 증분의 실제 matching-SHA CI 검증 및 수정, 실 OS·PID namespace·WAL restore drills.
2. 04.c..h(실제 영속성과 migration), 05.b..g(MTProto), 06.b..g(projection), 07.b..f(crypto) 등 설계된 상세 완료조건.
3. 하위 Phase마다 개별 commit + matching SHA pipeline + Draft MR로 누적. 실제 사용 가능 전환은 Phase 15 승인 후.

### Safety boundary

Detached synthetic database only; no automatic live SQLite writer, secrets, user Telegram account, Go replacement, data deletion, deployment or old user storage change. Go and Rust must never write to the same data dir or Telegram channel simultaneously.

### Phase 07.b2 verified implementation increment (2026-11-22)

[Draft MR !20](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/20) adds atomic 0600 private staging and no-clobber hard-link publishing for file encryption and decryption (not wired to app). Test contract covers 9MiB+, late tag corruption, truncated final tag, wrong keys, original-preservation, existing files, symlinks and public destination dirs. Latest implementation SHA `79b520bd26b3f604b332f697e8d2425945ff1ed3`; [GitLab pipeline #3098266896](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098266896) has separate `github-actions/ci-hub-request: success` status pointing to GitHub run 44952585082; GitLab CI itself only dispatches. Individual GitHub job logs and macOS runtime validation not independently available. **Partial phase, not production file encryption release.**

### Phase 07.d1 — sealed filename v1 Go parity (2026-11-22)

[Draft MR !21](https://gitlab.com/blond11qq/my_telegram_drive/-/merge_requests/21), source `44129cfaf6c94a77a01fa161a1add20e47f2f034`: Rust `encrypt_file_name`/`decrypt_file_name` with Go v1 HKDF salt+info, XChaCha20-Poly1305 AAD and 24-byte nonce, strict unpadded URL Base64, UTF-8 name bounds and redacted errors. Go original now exports five deterministic synthetic fixtures and GitHub/GitLab CI passes artifacts to Rust tests. [Pipeline #3098271844](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098271844) has separate `github-actions/ci-hub-request: success`, [GitHub run 44952988156](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/runs/44952988156). Specific job logs and macOS runtime unverified. No Argon2id vault lifecycle, Telegram persistence, live sync or full Phase 07.d support; **increment only**.
