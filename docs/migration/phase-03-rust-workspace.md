# Phase 03 — Cargo Workspace 및 헤드리스 Rust Skeleton

**목표**: Go 바이너리를 유지하며 Rust 구현을 **병행 추가**하고 CI/컴파일/리소스 수명 관리의 경계부터 검증한다.

## 신규 경로

- Cargo.toml / Cargo.lock / rust-toolchain.toml / rustfmt.toml / 필요 시 .cargo/config.toml.
- crates/tdrive-{types,config,storage,telegram,projection,sync,crypto,files,media,mount,service,api}/
- apps/tdrive-{server,cli,backup,desktop}/
- tests/{fixtures,compatibility,e2e}/, .gitlab-ci.yml (runner 확인 필요)
- 기존 go.mod, backend/*, cmd/*, Dockerfile/compose/installer는 **변경하지 않는다**.

## 구현 순서

1. Rust stable/MSRV와 Cargo edition 고정; Linux amd64/arm64, macOS Apple Silicon/Intel, Windows 빌드 feasibility 조사. GPUI git SHA는 Phase 12 전 확정 또는 optional crate로 격리.
2. Workspace.dependencies에 정확 버전/feature 명시, build --locked 재현 가능. Default members에서 GPUI 제외해 Linux headless가 윈도 의존성 없이 빌드되게 한다.
3. Domain newtype ChannelId(i64), MessageId(i64), FileId, FolderId, SecurePath, ErrorCode, ProgressEvent, RequestId. JavaScript 경계 64-bit ID는 decimal String.
4. Runtime owner: cancellation token, graceful shutdown + task joins, bounded queue, no zombie task. DI trait for Clock, RNG, Telegram, Storage, token verifier.
5. Secret file 0600/dir 0700, atomic temp write→sync→rename→parent fsync; Windows ACL은 전용 플랫폼 테스트. Tracing 민감 필드는 Redacted wrapper.
6. Skeleton CLI --version, 서버 health/status, metrics/tracing, fake TelegramPort, fake storage; 비구현 명령은 명확히 unsupported error.
7. 별도 임시 포트로 server 실행. 8080/운영 data dir, 기존 Go PID lock, Telegram session을 건드리지 않는다.
8. GitLab runner 구성: cargo fmt/clippy/test/build, dependency license/security check, Go/Bun regression 병행. 러너가 없으면 미실행으로 기록.
9. 별도 GPUI spike로 macOS와 Linux 최소 window를 실제 실행. headless CI의 cargo build에는 GPUI 포함하지 않는다.
10. CI artifact에 source SHA, rustc/cargo version, target triple, dependency lock hash를 넣는다.

## 예정 검증 명령

~~~sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --locked -p tdrive-server
cargo build --locked -p tdrive-cli
~~~

Desktop은 해당 OS GPU/display가 있는 job에서 별도로 테스트한다. 위 명령은 **계획**이며 이 문서 MR에서 실행하지 않았다.

## 완료 조건

- [ ] Rust workspace compile/test/lint 실제 실행 및 SHA 증거
- [ ] server/CLI headless builds on Linux amd64/arm64
- [ ] native GUI dependency가 server build graph에 없음
- [ ] typed ID/error/cancellation, logs redaction 단위 테스트
- [ ] Go/Docker/installer 동작 불변
- [ ] ADR-003/005/018의 미결정 사항과 담당자 명시

**롤백**: Rust 신규 항목과 해당 CI job만 비활성화. 운영 데이터/기존 이미지는 변경하지 않음.
