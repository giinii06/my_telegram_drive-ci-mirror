# Phase 08 — 파일 CRUD·대용량 전송·Revision·백업 에이전트

**선행** Phase 05~07 / **결과** Rust 전송 계층이 Go와 동일한 파일·원본 보존 계약을 충족. 원격 쓰기는 opt-in/시험 환경에서 점진적 활성화.

## 현재 Go 구현 위치

- backend/services/{file,folder,read,lifecycle}/; 특히 backend/services/file/upload.go
- backend/tgclient/{write_coordinator.go,upload_part_retry.go,download.go,range.go,idempotency.go}
- backend/projection/{multipart.go,file_revision_state.go,dirents,writeable*,hard_delete*,trash.go,restore_apply.go}
- backend/crypto/{stream.go,random_access.go,name.go}
- cmd/tdrive-backup/{main.go,client.go,scan.go,state.go}
- cmd/tdrive/web.go 의 upload worker, upload status와 file download.
- backend/mountwrite/*는 Phase 10에서 동일 commit 계약 재사용.

## 신규 파일 및 API 경계

- crates/tdrive-files/src/{upload.rs,download.rs,stream.rs,multipart.rs,revision.rs,commit.rs,move_copy.rs,trash.rs,progress.rs,temp.rs}
- crates/tdrive-service/src/{file_usecases.rs,folder_usecases.rs,backup_service.rs}
- apps/tdrive-backup/src/{config.rs,scanner.rs,journal.rs,client.rs,runner.rs}
- tests/compatibility/{file_operations.rs,upload_wire.rs,backup_journal.rs}
- tests/e2e/{upload_resume.rs,download_ranges.rs,archive_delete_safety.rs}

## 상세 구현 순서

1. **API 계약 동결**: Go의 업로드/다운로드/복사/이름변경/이동/삭제/복원/overwrite 정책, src/dst name conflict, parent absent, permission, encrypted destination 처리와 실제 오류 코드 정의.
2. **Read/Write state machines**: enum Pending→Preparing→Uploading→RemoteAccepted→Projecting→Committed/Failed/Cancelled. 메시지 전송은 로컬 트랜잭션이 아니므로 accepted 이후 crash가 발생할 수 있다는 계약을 명시.
3. **입력 보안**: 선택 경로 canonicalization, traversal/symlink/TOCTOU, 원본 변경(mtime/size/fd identity), 임시 파일 0600, deadline/cancel, 파일 경로·토큰 로그 redaction.
4. **Upload planner**: stored size vs plaintext size, Telegram 업로드 한도와 part 분할, single message vs multipart, 암호화 TDE1 overhead, 각 part 순번/uuid, final manifest. 전송 용량 추정·실제 bytes 차이 검증.
5. **Stable random ID**: 각 upload part 및 control 메시지에 재시도 동안 불변 random_id. FLOOD_WAIT/unknown receipt 시 무한 resend 및 새로운 object 중복 생성 금지.
6. **부분 실패 수습**: 일부 part만 생성된 경우 visible manifest 만들지 않음. orphan remote parts cleanup은 확증/별도 allow 정책으로, 성공된 다른 사용자 파일 메시지를 지우지 않도록 ID 집합 점검.
7. **Projection boundary**: 성공한 visible file op만 projection commit. batch는 DB에서 전부 적용/전부 실패, remote send는 atomic하지 않으므로 replay/sync 재수렴 가능하게 설계.
8. **Overwrite / revision**: logical file ID와 content_msg_id 구분, version/parent ID, optimistic concurrency, original revision rollback, trash/restore/hard delete의 scope와 confirm dialog.
9. **Download**: multipart stream assembly, byte-range 요청, encrypted random-access, remote retry, EOF 및 hash/size 확인. 가능한 경우 원자 temp→rename, overwrite policy, cancellation cleanup.
10. **UI progress**: per-file ID, sent/total, wait_seconds, stage, retryability. 지연/취소가 발생해도 다른 batch 파일 진행과 event ordering 유지.
11. **Backup separate app**: 기존 HTTP /api/upload와 /api/upload/status, /api/list 기반 동작 유지. Agent config(host/token/folder/remote, KeepOriginal, interval, settle, timeout) 및 journal JSON 구성을 동등하게.
12. **Archive deletion safety**: local file snapshot(size, mtime, optional hash) → upload → server complete → remote entry 검증(파일명·size 및 가능하면 hash) → source 재확인 → delete. 그 전에는 원본 유지. 최종 rename/삭제 사이 파일이 교체되면 삭제 금지.
13. **Backup journal**: write tmp + fsync + atomic rename, restart 후 already uploaded skip, 잘못된 JSON은 이름 변경 격리 후 **업로드한 것처럼 간주하지 않음**; 작업 concurrency와 duplicate backup state.
14. **대량 작업**: queue size cap, global maxConcurrentUploads, bounded open files, backpressure, per-folder scan safety, rename/move와 upload 경합, disk full/Telegram quota.
15. **기능 플래그**: 초기 read-only, 그 다음 신규 파일/디렉터리 create, 마지막 overwrite/delete/restore 순으로 gate 분리.

## 테스트 시나리오

| 오류/경쟁 | 기대 |
|---|---|
| multipart 2/3만 upload 후 crash | visible file 생성 안 됨, replay/cleanup 안전 |
| send accepted + receipt timeout | 같은 stable ID 재시도/수렴, 중복 visible object 없음 |
| wrong parent/duplicate sibling/path traversal | Go와 같은 안전한 오류 |
| plaintext/encrypted policy ambiguity | Go 정책에 따라 fail closed |
| 0 byte / large file / random range | hash와 byte-for-byte 동등 |
| download 취소 중 temp 존재 | local 완성 파일로 잘못 rename 금지 |
| backup 원격 상태 unknown/size mismatch | local original 보존 |
| backup journal corrupted | quarantine + crash, 원본 삭제 금지 |
| backup 원본 업로드 중 파일 변경 | 삭제 또는 uploaded 표기 금지 |
| vault lock in-flight | session invalidated, plaintext leak 금지 |

## 완료 조건

- [ ] single/multipart/encrypted/restore/trash/revision e2e with fake Telegram
- [ ] remote acceptance uncertainty/retry/restart injection 통과
- [ ] 원본 파일 삭제 mock은 **검증 완료 전 0회**
- [ ] backup journal Go↔Rust read/write compatibility 검증
- [ ] 성능·메모리·FD 수량 기준선 범위 비교
- [ ] destructive ops는 별도 승인까지 default-off

**롤백**: 새 Rust writer 비활성화, Go daemon + 기존 backup agent로 복귀. remote에 이미 존재한 메시지는 함부로 지우지 말고 sync/reconcile로 처리.
