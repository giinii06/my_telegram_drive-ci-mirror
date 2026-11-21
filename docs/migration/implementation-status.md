# Rust migration implementation status — updated 2026-11-22

현재 branch lineage: docs plan branch → Phase 04 → 05 → 06 → 07 → 08 → 09 → full parity plan. 원래 Go main은 수정/배포되지 않았으며 Rust 코드는 운영 서버에 연결되지 않았다.

**이 표의 "CI verified"는 해당 증분의 테스트만 통과했다는 뜻이고 phase 전체 완료가 아니다.** 모든 미구현 작업의 상세 목록은 [phase-ledger](phase-ledger.md)를 보라.

| Phase | 실제 Rust 증분 | GitLab 증거 | 전체 Phase 완료 |
|---|---|---|---|
| 00~03 | Go contract guard, curated TDX1 parser fixtures, Rust workspace + inspect CLI | inherited Phase 04 CI | 아니오 |
| 04.a | Go 생성 SQLite v16 snapshot read-only schema/integrity/row counts | b4733e9 / pipeline 3097711065 | 아니오 |
| 05.a | offline MTProto trait model + Go parity stable random_id | 9c4031c / pipeline 3097721220 | 아니오 |
| 06.a | 7 simple TDX1 writer formats + 14 Go parity cases | 3c45de1 / pipeline 3097739847 | 아니오 |
| 07.a | 8 MiB 제한 authenticated TDE1 read-only decrypt + 6 Go cipher fixtures | 89243d8 / pipeline 3097788263 | 아니오 |
| 08.a | backup deletion-permit decision type-state; unlink 미구현 | dc6e0d7 / pipeline 3097799701 | 아니오 |
| 09.a | Go-equivalent Range parser, 18 fixtures; media server 미구현 | 89fbdd1 / pipeline 3097808960 | 아니오 |
| 10~15 | production Rust WebDAV/server/GPUI/Docker/release/cutover 미구현 | 검증 증거 없음 | 아니오 |

사용자가 의존할 수 없는 것: **Rust Telegram 로그인/전송, 재생/replay projection writer, vault key lifecycle, file/backup I/O, HTTP/WebDAV server, native GPUI GUI, Rust Docker image, Go→Rust production cutover 전부 미구현.**

### 다음 작업 순서

1. [전체 실행 원장](phase-ledger.md)의 04.b Go parity folder listing 구현/검증.
2. 04.c..h(실제 영속성과 migration), 05.b..g(MTProto), 06.b..g(projection), 07.b..f(crypto) 등 설계된 상세 완료조건.
3. 하위 Phase마다 개별 commit + matching SHA pipeline + Draft MR로 누적. 실제 사용 가능 전환은 Phase 15 승인 후.

### Safety boundary

Detached synthetic database only; no automatic live SQLite writer, secrets, user Telegram account, Go replacement, data deletion, deployment or old user storage change. Go and Rust must never write to the same data dir or Telegram channel simultaneously.
