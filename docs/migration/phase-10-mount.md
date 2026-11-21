# Phase 10 — WebDAV / 파일시스템 마운트 / 쓰기 원자성

**선행** Phase 08~09 / **결과** 기존 WebDAV 파일 액세스와 선택적인 호스트 OS mount를 Go와 동일한 안전성으로 구현.

## 현재 파일과 중요한 정책

- backend/mountdav/server.go: capability 보호된 local WebDAV, 기본 read-only, 쓰기는 explicit opt-in.
- backend/mountcontroller/{controller.go,types.go}: stopped/preparing/attaching/mounted/draining/detaching/failed 상태기계.
- backend/mount{adapter,cache,content,fs,lifecycle,os,path,policy,safe,scratch,write}/*: remote fs, cache, invalidation, path mapping, write coordination.
- backend/mountpolicy/policy.go: policy 불명 시 plaintext 쓰기 거부.
- backend/daemon/{mount.go,mount_lifecycle.go,protocol.go}, cmd/tdrive/mount.go.
- Dockerfile/docker-compose.yml: TDRIVE_NO_OS_MOUNT=1; WebDAV 내부 연결은 사용.

## 목표 Rust 경계

- crates/tdrive-mount/src/{webdav.rs,capability.rs,fs.rs,path.rs,cache.rs,read.rs,write.rs,journal.rs,controller.rs,platform/{macos,linux,windows}.rs}
- crates/tdrive-api/src/mount_routes.rs
- tests/compatibility/{webdav_methods.rs,mount_state.rs,path_rules.rs}
- tests/e2e/{webdav_write_crash.rs,failed_detach.rs,remount.rs,mount_encrypted.rs}

## 자세한 작업 순서

1. **기존 mount behavior trace**: mountfs Go interfaces, GetAttr/Readdir/Open/ReadAt, write mode, Windows drive selection, unique relative paths, collision suffix mapping, hidden/system paths, symlink/traversal rules 스냅샷.
2. **Stage A read-only local WebDAV**: loopback bind, random per-session capability prefix, strict path validation, method/timeout/header size 제한, configured drive pinned, root path behavior. 권한 없는 원격 접근 불허.
3. **Content lifetime**: 파일 handle은 logical (channel,file,revision)로 pin; 백그라운드 update가 projection을 바꿔도 이미 열린 old reader의 동작은 Go contract와 비교.
4. **Cache/invalidation**: 원격 Telegram projection changed event 수신 후 directory snapshot과 negative cache 무효화. 절대 stale list로 존재하지 않는 파일을 유령 파일로 노출하지 않음.
5. **Write mode 보안**: writable mount는 기본 비활성화, authorized personal/shared drive와 암호화 정책 확인 후 enable. 서버 종료/lock·logout 중 새 write 허용 금지.
6. **Write state machine**: create/open→local scratch staging→fsync/close→upload parts→remote accepted→TDX1 fcommit/freplace/dcommit→projection→visible. 중간 임시 이름을 remote visible file로 오인시키지 않는다.
7. **Crash journal**: opaque op ID, targeted object revision, staging path, remote message IDs, cleanup intent를 기록; daemon kill/재시작 후 incomplete/uncertain op를 재검증하고 중복 commit하지 않는다.
8. **Path invariants**: duplicate sibling name, case-insensitivity(macOS/Windows), Unicode normalization, reserved characters, path separator, deeply nested folders, root empty parent, max filename, invalid symlink 처리.
9. **Delete/rename/move**: 멱등 op 및 revision 체크, tombstone/restore/hard-delete 정책, 충돌 발생 시 EEXIST/409 등 Go와 정렬된 사용자 오류.
10. **OS connector**: WebDAV read/write가 안정된 후 macOS Finder, Linux GVfs/FUSE, Windows WinFsp 또는 host OS helper의 실제 종속성 검증. GPUI window와 mount process lifecycle 분리.
11. **Container**: Docker의 TDRIVE_NO_OS_MOUNT=1 유지. GUI/권한 있는 host mount가 필요하다고 privileged Docker/host volumes를 요구하지 않는다.
12. **Mount controller states**: preparing→attaching→mounted→draining→detaching→stopped + failed. Start/Stop idempotent, conflicts, timeout, endpoint failure, auto recovery 상태와 health/event 보존.
13. **Security**: capability URL은 CLI status/log에 누출 금지, loopback route path disclosure 방지, cross-tenant drive ID check, path traversal/race and file permissions 테스트.

## 필수 장애 테스트

| 주입 장애 | 기대 |
|---|---|
| WebDAV PUT 중 client abort | partial data invisible / staged temp cleaned or recoverable |
| Upload success but local project failed | replay로 수렴, no duplicate remote |
| Scratch disk full | old file revision intact, safe error |
| Encrypted policy unknown | writable open 차단 |
| Vault lock during read/write | key lease revoke, dirty write fail closed |
| Duplicate sibling/case collision | 결정적 충돌 반환, 임의 파일 선택 금지 |
| concurrent rename/delete/upload | 원자적 name mapping, revision conflict 감지 |
| OS detach failure / forced shutdown | 상태 failed/draining, stale mount cleanup |
| Capability leak / remote listener | 권한 없는 접근 차단 |
| Headless container start | native mount 연결 시도 없음, internal WebDAV 사용 가능 |

## 완료 조건

- [ ] WebDAV method/response parity + fail-closed auth tests
- [ ] read-only 기능 기본값 유지
- [ ] writable path journal/restart/atomic visibility 검증
- [ ] macOS/Linux/Windows OS connector별 실제 host smoke
- [ ] container no-OS-mount regression
- [ ] path collision/Unicode/symlink and vault concurrency 안전

**롤백**: host OS attachment를 먼저 unmount/drain, Rust mount flag off, 기존 mount가 안전하게 제거되었는지 확인 후 Go만 재시작. 미완료 staged data를 임의 삭제하지 않는다.
