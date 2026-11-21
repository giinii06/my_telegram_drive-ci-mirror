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

Linux에서 브라우저 시스템 패키지도 필요하면 `browser:install` 뒤에 `--with-deps`를 붙이세요. CI 명령은 [.github/workflows/ci.yml](.github/workflows/ci.yml)에 있습니다.

`compile: signal: killed`가 발생하면 Docker 메모리·스왑을 확인하세요. 스왑 없는 2GB 환경에서는 빌드가 실패했습니다.

## Rust 병렬 CI

Rust 브랜치에서 GitHub Actions + GitLab Oracle ARM64 검증을 병행하려면 [병렬 CI 운영 문서](docs/migration/github-parallel-ci.md)를 참고하세요. GitLab 커밋만으로 GitHub CI가 실행되는 것은 아니며 실제 GitHub 브랜치 미러링이 필요합니다.

## 유지할 규칙

- JS 의존성은 `bun.lock`으로 고정하고 `bunfig.toml`의 Bun 실행 설정을 유지하세요.
- 컨테이너의 운영체제 마운트 생략과 내부 WebDAV 연결을 유지하세요.
- 인증 정보·웹 토큰·마운트 접근 URL을 출력하거나 커밋하지 마세요.
- 삭제·재생성·전체 로그아웃의 확인 절차를 유지하세요.

[라이선스](LICENSE)
