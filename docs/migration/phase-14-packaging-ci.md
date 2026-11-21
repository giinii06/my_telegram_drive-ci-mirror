# Phase 14 — GitLab/GitHub CI, 네이티브 패키징, 공급망 및 출시 게이트

**선행** Phase 11~13 / **결과** Rust 서버/데스크톱/CLI/백업의 플랫폼별 산출물이 실제 빌드·실행 검증과 업그레이드 테스트를 통과한다.

## 기존 구조

- .github/workflows/ci.yml: Go test/vet/build + Bun lint/Playwright + Docker build.
- .github/workflows/installers.yml: deb/dmg/Windows exe/backup exe/Android APK release.
- packaging/{linux,macos,windows,android}/* 및 packaging/README.md.
- Linux Debian systemd service, macOS dmg + command helper, Windows Inno Setup, Android app service.
- GitLab repo의 기준 SHA에는 .gitlab-ci.yml이 없고 **shared_runners_enabled=false**로 표시됨. runner/CI quota/ARM64 실행 가능성부터 검증해야 한다.

## 새 CI 단계 (각 MR에서 job별 실제 증거)

1. **Docs-only**: Markdown links/format, structural cross references, Mermaid syntax/링크 점검. Rust/Go unit tests는 docs-only 조건으로 skip될 수 있으나 '통과'로 허위 보고하지 않는다.
2. **Rust static**: cargo fmt/clippy, cargo test --workspace --locked, custom integration, dependency audit/license, cargo deny(도구/정책 확정 후), SBOM.
3. **Go baseline 유지**: go test/vet/build, Bun lint/unit/browser Playwright. Go↔Rust golden fixtures replay/diff와 모든 P0 contracts.
4. **Security**: HTTP auth/session/path, encryption tamper, secrets-in-log, DB concurrent writer, archive deletion safety, supply-chain checks.
5. **Linux target**: amd64/arm64 cargo build + Docker buildx + Compose smoke, limited memory/OOM test, UID migration. Apple runner/Linux runner의 실행 아키텍처를 명시.
6. **macOS GPUI**: host-native Apple Silicon/Intel build, application launch, window/IME/file chooser smoke, UI screenshot/keyboard integration. Gatekeeper/notarization signing 여부 별도 표시.
7. **Linux GPUI**: X11/Wayland native/test-display smoke. 단순 cargo check를 창 실행 성공으로 취급하지 않는다.
8. **Windows**: pinned GPUI가 지원되면 native exe + smoke, MSVC dependency, named pipe ACL, Inno installer. 지원 불가면 기존 Go/웹 fallback package를 명시적으로 유지.
9. **Android**: 기존 Go/WebView APK build+smoke는 Rust GPUI와 별개 유지. Android Rust로 전환하는 별도 Phase/MR이 승인되기 전 제거 금지.
10. **Release hardening**: reproducible tags, Cargo.lock, rustc/GPUI/Telegram crate SHAs, license/CVE report, checksums, optional artifact signing, OCI image digest, rollback release link.

## 추천 job DAG (제안)

~~~mermaid
flowchart TD
  VERIFY[metadata + docs] --> STATIC[Rust fmt clippy tests]
  VERIFY --> LEGACY[Go + Bun + Playwright]
  STATIC --> CONTRACT[Go-Rust golden compatibility]
  LEGACY --> CONTRACT
  CONTRACT --> SEC[security + crash fault-injection]
  SEC --> LINUX[Linux amd64/arm64 server+Docker smoke]
  SEC --> MAC[macOS GPUI native UI]
  SEC --> WIN[Windows guarded / fallback]
  SEC --> MOBILE[Android legacy regression]
  LINUX --> RELEASE[release candidate: manual approval]
  MAC --> RELEASE
  WIN --> RELEASE
  MOBILE --> RELEASE
~~~

실제 DAG는 runner availability에 맞춰 정리. 결제 크레딧 소진/런너 연결 불가와 코드 오류를 반드시 구분한다. CI job가 pending/stuck이면 통과로 표기하지 않는다.

## 네이티브 패키징 순서

- **Linux**: systemd service ExecStart를 Rust binary로 변경하기 전에 기존 config dir/volume/port/permissions, package postinst/prerm/remove behavior, on-host socket+WebDAV, deb upgrade/downgrade 검증.
- **macOS**: GPUI .app/.dmg bundle, 애플 실리콘 + Intel target, 앱 종료와 daemon service 독립성, 기존 Install TDrive.command/Open TDrive.command migration, DMG mount smoke. Code signing/notarization 미지원이면 README에 정확히 알림.
- **Windows**: Rust server+CLI/backup exe, named-pipe ACL/SID, firewall/listen choices, Inno Setup, HKCU Run registration, uninstall retention; GPUI는 지원 확인 전 별도 fallback.
- **Android**: 기존 APK/Go shared library, WebView startup/permission/foreground service, 사용자의 저장 데이터 유지. Rust로 통합하려면 Android target/jni ABI/crypto parity를 별도 프로젝트로.
- **Docker**: OCI amd64/arm64 images + Compose, tailscale sidecar, persistent volumes, health/readiness; GPUI 없음.
- **Backup installer**: cfg/journal paths and record compatibility, 원본 삭제/재시작 E2E, Windows start/stop/status/log entries and uninstall.

## MR/CI 수락 체크리스트

- [ ] source SHA와 pipeline SHA가 정확히 같음
- [ ] job별 실제 결과 링크, not-run/stuck/failed 명확
- [ ] Rust + Go golden tests P0=0, unit/integration/E2E/platform smoke
- [ ] dependency licenses/CVEs/FFI/unsafe reviewed
- [ ] signed/published release artifacts의 SHA-256 및 provenance
- [ ] ARM64 container 실제 실행, Win/mac/Linux native runtime checks
- [ ] 구 설치본 업데이트/삭제/롤백 시 데이터 보존
- [ ] Android 유지 또는 변경에 대한 승인된 지원표

## GitLab 특이사항

- 현재 GitLab은 main과 별도 MR runner 작업이 있음. 실제 MR의 CI를 새로 구성할 때 보유 runner capacity, tag, Linux/ARM64 job 매칭, 보호 브랜치 권한, GitHub mirroring 영향 조사.
- 미러링된 repository에서 GitLab·GitHub에 각자 자동 릴리스 푸시를 구성하면 중복/레이스 가능. **릴리스 write authority를 하나로 지정**.
- 실패하는 CI job은 실제 로그를 읽고 원인별로 patch. 무조건 rerun/pass로 표시 금지.

**롤백**: 변경된 runner/release job 비활성화, 이전 Go packaging workflow 유지. 이미 배포한 image/installer에 데이터 삭제가 없었는지 검증하고 다른 platform은 단계적으로 복귀.
