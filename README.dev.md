# TDrive 개발자 가이드

설치와 로그인은 [사용자 가이드](README.md)를 참고하세요.

## 환경과 구조

Docker Compose v2, Go 1.25.13 이상, Bun 1.4.3을 사용합니다.

- `cmd/tdrive` — CLI와 웹 서버
- `cmd/tdrive/webui` — 바이너리에 포함되는 HTML, Bun 검사, Playwright 테스트
- `backend` — 데몬, Telegram 연결, 파일 저장, 내부 WebDAV
- `docker/entrypoint.sh` — 데몬 시작, 드라이브 선택 대기, 웹 서버 실행

앱은 Docker에서 실행하고 JavaScript 도구는 Bun으로 실행합니다. 웹과 CLI는 같은 데몬을 사용합니다.

## 실행과 검증

명령은 저장소 최상위에서 실행하세요. HTML을 수정해도 이미지를 다시 빌드해야 합니다.

```sh
docker compose up -d --build

go test ./cmd/tdrive ./backend/...
go vet ./cmd/tdrive ./backend/...

bun install --cwd cmd/tdrive/webui --frozen-lockfile
bun run --cwd cmd/tdrive/webui lint
bun run --cwd cmd/tdrive/webui browser:install
bun run --cwd cmd/tdrive/webui test
```

Linux에서 브라우저 시스템 패키지도 필요하면 `browser:install` 뒤에 `--with-deps`를 붙이세요.

`compile: signal: killed`가 발생하면 Docker 메모리·스왑을 확인하세요. 스왑 없는 2GB 환경에서는 빌드가 실패했습니다.

## CI 실행과 결과 확인

GitLab `main`에 푸시하면 GitLab이 공개 GitHub 미러를 동기화합니다. GitHub-hosted Runner가 중앙 `ci-hub`의 공통 워크플로에서 Go 빌드와 테스트를 실행합니다.

- [GitHub Actions 실행 로그](https://github.com/giinii06/my_telegram_drive-ci-mirror/actions/workflows/gitlab-bridge.yml)를 확인합니다. 즉시 재실행하려면 해당 실행의 **Re-run jobs**를 선택합니다.
- GitLab 커밋 상세에서 같은 SHA의 `github-actions/ci-hub` 상태를 확인합니다. GitHub에서 완료하면 호스팅된 컨트롤러가 결과를 GitLab에 반영합니다.

Docker와 웹 UI 검증은 같은 미러의 `Docker and application checks` 워크플로에서 확인합니다.

### ChatGPT 웹에서 CI 요청

GitLab 플러그인에서 이 프로젝트의 브랜치 또는 태그를 지정해 파이프라인을 생성합니다. GitHub 컨트롤러가 해당 SHA를 검사한 뒤 필요한 이력을 공개 미러에 전달하고 CI를 예약합니다. GitLab 파이프라인에서 `github-actions/ci-hub-request`의 상태와 GitHub 실행 링크를 확인합니다. [프로젝트 공통 CI 안내](https://github.com/giinii06/ci-hub#chatgpt-웹에서-ci-요청)

### CI 병렬 실행

- Pmanager와 TDrive의 CI는 동시에 실행할 수 있습니다. 같은 프로젝트의 여러 푸시가 각각 실행을 시작하면 그 실행들도 겹칠 수 있습니다. 사용 가능한 러너가 모두 사용 중이면 실행은 대기열에 들어갑니다. [워크플로 동시 실행](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)
- 현재 공통 워크플로는 실행마다 `build` 작업 하나에서 Go 빌드와 테스트를 순서대로 실행합니다. 한 실행 안에서 병렬로 처리하려면 `ci-hub`에서 독립 검사를 별도 `jobs`로 나눕니다. 같은 검사를 여러 버전이나 운영체제에서 실행하려면 `strategy.matrix`를 사용합니다. 선행 결과가 필요한 작업에는 `needs`를 지정합니다. [작업 의존성](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-jobs) · [매트릭스 실행](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations)

## 유지할 규칙

- JS 의존성은 `bun.lock`으로 고정하고 `bunfig.toml`의 Bun 실행 설정을 유지하세요.
- 컨테이너의 운영체제 마운트 생략과 내부 WebDAV 연결을 유지하세요.
- 인증 정보·웹 토큰·마운트 접근 URL을 출력하거나 커밋하지 마세요.
- 삭제·재생성·전체 로그아웃의 확인 절차를 유지하세요.

[라이선스](LICENSE)
