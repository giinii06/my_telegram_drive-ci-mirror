# Phase 04 — Rust SQLite / 설정 / 기존 데이터 무손실 이관

**선행** Phase 00~03 / **결과** Rust가 SQLite v16을 안전하게 읽고 검증·복구 가능. 이 Phase에서 실제 사용자 데이터를 자동 변경하거나 Go 서버를 교체하지 않는다.

## 현행 코드의 근거

- backend/fstructs.go: process-wide SQLite 핸들과 단일 connection, PRAGMA 초기화.
- backend/projection/schema.go: currentSchemaVersion 16, EnsureSchema, MigratePersonalChannel, 폴더/파일 테이블 생성과 이전 구조 재형성.
- backend/projection/writable_schema.go: revision/dirent/file_revisions/operation migrations.
- backend/projection/{gallery_schema.go,media*,rendition*,trash.go,hard_delete*}: projection 부속 테이블·상태.
- backend/projection/log.go: replay_log 원자 쓰기, raw header SHA-256, tamper/reject 경로.
- backend/datadir/datadir.go, backend/auth/{private_file.go,config.go,session_storage.go}: config, secrets, cache paths.
- backend/processlock/lock.go: 데이터 디렉터리별 단일 소유자.

## 신규 구현 파일/모듈

- crates/tdrive-storage/src/{lib.rs,connection.rs,schema.rs,migrations.rs,transactions.rs,projection_store.rs,backups.rs,integrity.rs}
- crates/tdrive-config/src/{paths.rs,private_file.rs,settings.rs,session_paths.rs,lock.rs}
- tests/compatibility/{sqlite_schema.rs,migration_from_go.rs,replay_consistency.rs}
- tests/fixtures/sqlite/{v16-empty,v16-populated,v16-encrypted,v16-writable,legacy-v*,interrupted-*} (fixture만)
- 운영 작업용 apps/tdrive-server의 migrate check/dry-run command 또는 독립 오프라인 검증 subcommand.

## 구현 작업(순서 엄수)

1. **정확한 SQL 추출**: Go에서 모든 CREATE/ALTER/INDEX/UNIQUE/PRAGMA 및 schema_version 변화 경로를 도출. 새 Rust migration version을 v16처럼 보이게 임의로 재번호하지 않는다. sqlite_master와 table_info 기반으로 비교.
2. **DB driver 평가**: rusqlite/sqlx 중 transaction, connection pool, WAL, serialized writer, busy_timeout, backup API, cross compile을 시험. SQLite driver가 서로 다른 version에서도 데이터 무결성과 collation을 지키는지 확인.
3. **읽기 전용 판독 구현**: copy된 Go DB에서 channels, replay_log, files, folders, dirents, revision, encryption configs, gallery, rendition, trash, pending_joins을 SELECT해 normalized hash 비교.
4. **path resolver**: ~/.config/TDrive 및 ~/.cache/TDrive /data 오버라이드; 디렉터리 0700, 비밀 0600; symlink hijack 및 다른 계정 파일 접근 검사. Windows 저장 경로는 Go 실제 생성 경로를 fixture에서 비교.
5. **config schema**: imp_config.json, personal drive selection, session, web.token, vault.pass, backup journal에 대해 missing/corrupt/old-version/permission-error의 세 상태 이상을 명시. 파싱 실패 시 **빈 새 계정 설정으로 덮어쓰지 않는다**.
6. **single writer**: Go PID lock만 신뢰하지 말고 lockfile/OS advisory lock/SQLite busy를 조합하는 정책을 ADR-008에 남긴다. Go와 Rust를 동시에 띄워 같은 DB를 쓸 수 없어야 한다. Docker 데이터 볼륨의 host sharing 시나리오 포함.
7. **스냅샷**: WAL checkpoint 또는 SQLite backup API로 일관된 복사본 생성. raw cp만으로 WAL 누락시키지 않음. 파일 owner, ACL, mode, hash, free disk, schema version을 manifest로 생성.
8. **migration dry run**: 새 code가 clone DB에만 새 버전의 migration을 적용한 뒤 PRAGMA integrity_check / foreign_key_check 및 핵심 row count/hash/invariants를 비교. 실행 전후의 partial migration, I/O interruption 실험.
9. **atomic activation**: 운영 적용 시 Go daemon 정지→lock 확인→일관 backup→Rust migrate clone→검증→원자적 디렉터리/파일 교체 또는 서비스 포인터 변경→Rust 실행. 실제 atomicity는 FS별 테스트 필요.
10. **downgrade 정책**: Go v16이 읽을 수 없도록 바뀐 schema는 rollback 시 스냅샷 복원만 가능. DB 파일과 Telegram 원격 쓰기의 시간 차를 고려해 rollback 직후 Go incremental sync로 수렴시킨다. 단순 버전 값을 낮추지 않는다.
11. **corruption recovery**: failed migration에 대한 quarantine 및 현재 데이터 무수정. auto-repair는 별도 복사본 검증과 owner 승인 전 금지.

## 보존해야 할 불변식

- files/folders의 channel-scoped identity, root parent empty, d:/f: prefix, unique active dirents, content_msg_id, revision, multipart, encryption hint, replay logs.
- replay_log와 projection 변경은 **같은 DB transaction**에서 원자적, 텍스트 SHA-256 tamper detection.
- Personal channel schema migration은 실제 channel ID가 알려진 뒤에 발생하는 현재 시맨틱 유지.
- 실행 프로세스 하나가 writer 소유. 화면 단독, backup, CLI는 DB 파일 직접 열지 않는다.
- 암호화 정책이 DB에 없다는 사실만으로 평문 쓰기를 허용하지 않는다(Phase 07).

## 주요 테스트 케이스

| 케이스 | 입력 | 기대 |
|---|---|---|
| Go v16 existing | populated DB copy | 핵심 row counts 및 sorted data hash 같음 |
| Legacy migration | 지원 버전별 clone | 단계적 upgrade, orphan/cycle 정책 보존 |
| Mid-transaction power loss | fault injection | 기존 DB 무수정 또는 복원 가능 |
| WAL copied incorrectly | WAL active DB | 안전한 backup 방식만 수용 |
| Competing Go and Rust | 동일 디렉터리 | 두 번째 writer 명확히 실패 |
| Missing session | 빈 config | 새 사용자로 데이터 덮어쓰기 금지 |
| Unsupported future schema | schema > supported | 읽기/쓰기 금지, 설명 가능한 오류 |
| Corrupt file / low disk | DB copy | fail-safe + backup intact |
| Directory symlink | 설정 경로 조작 | 외부 경로에 secret write 금지 |

## 완료 조건

- [ ] 동일 schema v16 read-only parity, 모든 migration path 테스트
- [ ] writable DB의 전/후 integrity_check + foreign_key_check
- [ ] migration dry-run / snapshot restore / rollback drill 실제 수행
- [ ] raw copy / WAL / crash / 두 writer / unsupported schema cases 통과
- [ ] secret file/dir permissions, symlink, data root 보존
- [ ] version bump가 필요한 경우 ADR과 backward compatibility 승인을 받음

**중단 조건**: row 하나라도 비의도적으로 유실·중복되거나 복원 검사 실패 시 전환 중지. 기존 Go 바이너리와 스냅샷을 그대로 보존.
