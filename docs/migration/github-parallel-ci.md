# GitLab + GitHub 병렬 CI — Rust migration

이 문서는 실제로 확인한 구성을 기준으로 한 운영·검증 가이드다.

## 확인한 사실
- GitLab은 코드와 커밋을 관리하는 기준 저장소다. GitHub 공개 미러는 giinii06/my_telegram_drive-ci-mirror.
- main에 있는 .github/workflows/gitlab-bridge.yml은 push/main → giinii06/ci-hub Go CI만 호출한다.
- GitLab main 커밋 d6ec2e70에는 github-actions/ci-hub success 외부 커밋 상태가 실제로 존재한다.
- 기능 브랜치가 GitHub에 자동 미러되는지와 신규 workflow가 실행되는지는 아직 확인되지 않았다. GitHub 플러그인 없이 GitLab 연결만 사용해 작업한다.

## 병렬 DAG
- legacy-contracts, rust-style, go-fixtures, legacy-go-tests, web-checks: 독립 실행 가능한 5개 GitHub hosted ubuntu-24.04 job.
- rust-contracts: go-fixtures 성공 후 synthetic artifact를 다운로드하여 Rust 1.92 clippy --locked 및 cargo test --locked 실행.
- GitLab oracle-arm64 파이프라인은 따로 유지한다. GitHub ubuntu-24.04는 일반적으로 amd64여서 이중 아키텍처 테스트에 유용하다.
- 실제 병렬 실행 수와 대기열은 GitHub hosted runner 및 Actions 요금제의 한도에 따라 달라진다. GitHub job들의 순차 step 자체는 병렬이 아니다.

## 실제로 작동시키기 위한 확인 절차
1. GitLab Settings → Repository → Mirroring repositories: 대상 GitHub와 push mirror의 대상 branch 범위를 확인한다. Mirror only protected branches가 켜져 있으면 비보호 refactor/**, docs/**, ci/**는 미러되지 않을 수 있다.
2. 모든 브랜치 미러링을 허용할 때 공개 GitHub에 코드·커밋이 노출된다. 기밀 및 비밀정보가 없는지 먼저 검토한다.
3. GitHub CI mirror 저장소에 refactor/rust-phase-04d-snapshot-digest 또는 ci/github-parallel-rust-20261122가 실제 존재하고 GitLab과 같은 40자리 SHA인지 확인한다.
4. Actions → Rust migration parallel CI에서 GitHub push 이벤트가 발생했는지, 그리고 5개 독립 job과 fixture 의존 Rust job이 실행됐는지 확인한다.
5. GitLab MR에서 정확히 같은 SHA의 6개 CI job을 따로 확인한다. 기존 github-actions/ci-hub 외부 상태 보고는 main의 Go workflow에 대해 검증됐을 뿐 Rust feature branch에는 자동 적용된다고 가정하지 않는다.
6. workflow_dispatch 수동 실행은 GitHub 기본 브랜치에도 워크플로 정의가 있어야 작동할 수 있다. 이번 분기 작업은 미러 push를 우선 사용한다.

## 안전성
- 워크플로 권한은 contents: read만 사용한다. secrets, 실제 Telegram 세션, 사용자 SQLite 파일, 배포 토큰 및 외부 네트워크 운영 호출은 없다.
- artifact는 Go가 새로 만든 synthetic v16 DB/TDE1 fixture만 포함하며 보존 기간은 1일이다.
- GitHub Actions가 실제 돌았다는 근거로 GitHub run URL과 동일 SHA가 필요하다. GitLab에 워크플로 YAML만 커밋한 상태는 미실행으로 기록한다.
- 원본 main 배포 GitHub Actions ci.yml publish는 main 전용이고 이 Rust CI workflow에는 publish job이 없다.
