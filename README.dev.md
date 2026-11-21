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
- GitLab 커밋 상세에서 같은 SHA의 `github-actions/ci-hub` 상태를 확인합니다. 상태 보고는 브리지가 실행 중일 때 5분 주기로 반영됩니다.

Docker와 웹 UI 검증은 같은 미러의 `Docker and application checks` 워크플로에서 확인합니다.

## 유지할 규칙

- JS 의존성은 `bun.lock`으로 고정하고 `bunfig.toml`의 Bun 실행 설정을 유지하세요.
- 컨테이너의 운영체제 마운트 생략과 내부 WebDAV 연결을 유지하세요.
- 인증 정보·웹 토큰·마운트 접근 URL을 출력하거나 커밋하지 마세요.
- 삭제·재생성·전체 로그아웃의 확인 절차를 유지하세요.

[라이선스](LICENSE)
