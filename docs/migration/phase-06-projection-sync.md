# Phase 06 — TDX1 이벤트, Projection, Sync, Recovery

**선행** Phase 04~05 / **결과** Telegram 원격 메시지로부터 동일한 로컬 파일 트리를 구축하고 오프라인·충돌·재접속에서 수렴한다.

## Go 기준 파일 / 계약

- backend/projection/{types.go,wire.go,apply.go,log.go,schema.go,rebuild.go,restore_apply.go,writable_apply.go,writable_schema.go,namespace.go,trash.go,hard_delete*.go,reconcile.go}
- backend/sync/{sync.go,difference.go,parse.go,initial_scan_spool.go}
- backend/livesync/{activity.go,coordinator.go}
- backend/core/{engine.go,projection_change.go}
- backend/tgclient/client.go
- TDX1 header, sorted (channel_id,msg_id), replay_log first_seen_hash SHA-256, schema 16, object prefix d:/f:, root "", ProtocolVersion 6은 독립 계약.

## 신규 대상

- crates/tdrive-projection/src/{wire.rs,types.rs,parser.rs,apply.rs,store.rs,replay.rs,rebuild.rs,writable.rs,trash.rs,revision.rs}
- crates/tdrive-sync/src/{history.rs,incremental.rs,initial.rs,difference.rs,spool.rs,live.rs,reconcile.rs,retry.rs}
- crates/tdrive-service/src/{engine.rs,operations.rs,events.rs}
- tests/compatibility/{tdx_roundtrip.rs,projected_db_diff.rs,history_replay.rs}
- tests/e2e/{offline_resume.rs,conflict_writes.rs,projection_rebuild.rs}

## 상세 전환 순서

1. **정규 wire model**: 모든 OpType + 필수/선택 필드, string escaping, URL/base64 encoding, filename encrypted envelope, versioned payload. Go golden의 Format/Parse와 동등한 canonical output.
2. **불변식**: parent empty string, d:<uuid>/f:<msg_id>; 형제 동일 이름 허용하되 path lookup ambiguity error, cycle reject, tomb/rmdir idempotent, unknown newer op의 명시적 skip/reject semantics.
3. **순수 projection**: 입력 tuple(channel,msg,op,actor)과 기존 DB로만 결과 생성. wallclock, Telegram network, RNG 참조하지 않는다. 단일 writer 및 per-channel 순차 적용.
4. **replay transaction**: raw header SHA-256, 기존 hash와 차이 시 replay_log_tamper 기록, 원본을 정본으로 유지. parse/apply reject는 audit, 성공/실패에 따라 정해진 rollback. channel/msg 중복 replay는 변화 없음.
5. **writable version**: fcommit/freplace/dcommit/relocate/trash/harddel/untrash와 op_id, operation epoch/revision, content_msg_id, upload_uuid, multipart manifest, file_revision, per-folder revision. Go current apply policy에 대한 fixture 기반 적용.
6. **전송과 projection 분리**: send → local projection은 remote atomic transaction이 아님. send 성공+projection 실패면 remote 정본은 유지되어 재동기화로 회복; 유저에게 중복 send를 요청하지 않는다.
7. **Initial scan**: history 계획(count→apply), spool streaming/bounded memory, strictly ascending msg_id, progress pages/messages count와 unknown totals 처리, interrupted scan checkpoint와 atomic publish.
8. **Incremental scan**: high-watermark/pts, channel difference, newest history, stale access_hash, deleted remote entries, replay status checks, missing message reconciliation.
9. **Live sync**: Telegram update는 doorbell/invalidation only. channel별 debounce, bounded nonblocking queue, dropped signal metrics, backstop interval, timeout/stop/resume. 원격 메시지를 UI 이벤트에서 직접 DB apply하면 안 됨.
10. **Projection rebuild**: 기존 replay_log에서 channel별 전체 rebuild, 256 row 정도 bounded batch(현재 Go 기준), 실패하면 원본 트리 상태 보존; tamper/reject 사유 측정.
11. **Multiple concurrent writers**: 채널별 mutex와 global upload limit, outbox/retry idempotency, stale operation revision, single data-dir daemon owner. Go와 동일 Telegram 채널을 동시에 쓰는 dual-writer 검증은 별도 격리 실험에서만.
12. **Hard delete**: remote deletion 전에 intent/target set/commit recoverability를 검사. 사용자 확인, preview, dry-run, feature flag. 잘못된 삭제 요청을 성공으로 응답하지 않는다.
13. **Event DTO**: drive_scan_progress, live_sync_* 및 projection invalidate; JS channel_id는 decimal string, event ordering/cancellation 보존.

## 최소 테스트

| 테스트 | 기대 |
|---|---|
| 같은 채널/msg 재수신 | replay row 및 projection 불변 |
| 같은 message id 다른 header | tamper log 생성, 원본 유지 |
| out-of-order / duplicate history | strict ascending apply / correct watermark |
| missing/cyclic parent | Go와 같은 error 또는 retained anomaly |
| send success + DB failure | 다음 history replay 후 수렴 |
| interrupted initial scan | 이전 readable snapshot 유지, 재시작 후 완료 |
| FloodWait/cancel/reconnect | 대기 표시, no deadlock, bounded retry |
| accepted remote write but reply lost | stable random_id, exactly-once logical projection |
| hard delete interrupted | remote/local double delete 없이 재개 또는 blocked |
| simultaneous vault lock/rename | fail-closed 보존 |

## 완료 조건

- [ ] 모든 TDX1 canonical fixture 및 legacy op types equal
- [ ] schema v16에서 projection/db differential tests 통과
- [ ] rebuild + initial/incremental/live sync + crash/restart/reconcile 재현
- [ ] operation id와 stable random_id의 재시도 안전성
- [ ] remote destructive actions default-off 유지, review 후 활성화
- [ ] 로그에 민감한 message/caption 및 secret 노출 없음

**롤백**: Rust writer 기능 off 후 해당 DB snapshot 복구/Go 재동기화. 이미 Telegram에 전송된 메시지를 rollback이 자동 취소한다고 가정하지 않는다.
