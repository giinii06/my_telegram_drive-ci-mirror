# TDrive

Telegram에 파일을 저장하고 웹에서 관리하는 드라이브입니다. Docker Compose로 실행합니다.

코드 수정과 테스트는 [개발자 가이드](README.dev.md)를 참고하세요.

## 한 줄 설치

Docker가 설치된 **Linux 서버**에서 실행합니다. Tailscale 인증 키는 [관리 화면](https://login.tailscale.com/admin/settings/keys)에서 발급합니다.

```sh
curl -fsSL https://raw.githubusercontent.com/giinii06/my_telegram_drive/main/install.sh | bash
```

설치 스크립트가 저장소를 `~/.tdrive`에 복제하고 `.env`를 만든 뒤 이미지를 내려받고(없으면 로컬 빌드), 웹 주소를 출력합니다. Tailscale auth key만 터미널에서 입력하고, 나머지 Telegram 설정은 브라우저에서 진행합니다: API ID·hash, 전화번호, 로그인 코드, 2FA, 드라이브 선택.

`~/.local/bin`에 `tdrive` 명령이 등록됩니다. PATH에 없으면 설치 마지막에 안내가 출력됩니다.

```sh
tdrive status
tdrive drives
tdrive help
```

- Docker가 없으면 `curl -fsSL https://raw.githubusercontent.com/giinii06/my_telegram_drive/main/install.sh | TDRIVE_INSTALL_DOCKER=1 bash`로 설치를 시도할 수 있습니다.
- 비대화형 환경에서도 설치는 끝나고, 온보딩은 웹에서 진행합니다.
- 온보딩은 Telegram 로그인 전에만 열리며, Tailscale로 연결된 기기에서 접속해야 합니다.
- 로그인을 10분 안에 완료하지 않거나 취소하면 브라우저에서 다시 시도할 수 있습니다.
- 설치 경로는 `TDRIVE_HOME`, 명령 위치는 `TDRIVE_BIN_DIR`로 바꿀 수 있습니다.
- 현재 compose는 `/dev/net/tun`이 필요한 Tailscale 사이드카를 쓰므로 Linux에서만 동작합니다.
- 웹은 Tailscale에 연결된 기기에서 `http://<tailscale-ip>:8080`으로 접근하며 별도 로그인이 없습니다.
- 이미지는 `ghcr.io/giinii06/my_telegram_drive:latest`(linux/amd64)를 사용합니다. GHCR 패키지는 첫 퍼블리시 때 비공개가 기본이므로, 저장소의 Packages 설정에서 public으로 바꾸면 익명 pull이 빨라집니다. 비공개이거나 pull에 실패하면 로컬 빌드로 넘어갑니다.
- arm64 서버는 배포 이미지가 없어 자동으로 로컬 빌드합니다. 다른 태그를 쓰려면 `TDRIVE_IMAGE`를 지정하세요.

수동 설치와 이후 관리 명령은 아래에 정리되어 있습니다.

## Ubuntu Server에 처음 설치하기

서버에 Git, Docker Compose v2, nano가 필요합니다. 아래 명령은 Ubuntu Server에 접속한 터미널에서 실행하세요.

### 1. 저장소 받기

```sh
git clone https://github.com/giinii06/my_telegram_drive.git
cd my_telegram_drive
```

공개 저장소이므로 누구나 복제할 수 있습니다.

### 2. .env 설정하기

[Tailscale 인증 키](https://login.tailscale.com/admin/settings/keys)를 발급받은 뒤 설정 파일을 만드세요.

```sh
cp .env.example .env
nano .env
```

`TS_AUTHKEY`에 발급받은 키를 넣으세요. 나머지 값은 그대로 두면 됩니다.

```dotenv
TS_AUTHKEY=tskey-auth-발급받은키
```

nano에서 `Ctrl+O` → `Enter`로 저장하고 `Ctrl+X`로 닫으세요. 이후 명령도 `my_telegram_drive` 폴더에서 실행합니다.

### 3. 컨테이너 실행하기

```sh
docker compose up -d --build
```

### 4. Telegram 로그인하기

[Telegram API ID·hash](https://my.telegram.org/apps)를 발급받으세요. 컨테이너 실행 후 아래 5번의 웹 주소를 열면 브라우저에서 API ID·hash와 로그인을 입력할 수 있습니다. CLI로 설정하려면 다음 명령을 실행하세요.

```sh
docker compose exec tdrive tdrive setup
docker compose exec tdrive tdrive login
docker compose exec tdrive tdrive drives
```

`drives` 출력에 `*`가 없으면 사용할 드라이브를 선택하세요.

```sh
docker compose exec tdrive tdrive drive use '드라이브 이름 또는 ID'
```

### 5. 웹 화면 열기

```sh
docker compose exec tailscale tailscale ip -4
```

Tailscale에 연결된 기기에서 `http://<출력된-IP>:8080`을 여세요. 웹 서버는 컨테이너와 함께 시작되며, 로그인 전에는 온보딩 화면이 표시됩니다. 웹에서 업로드·다운로드·검색·폴더 관리·미리 보기를 사용할 수 있습니다.

## 이후 실행과 관리

```sh
docker compose up -d                 # 시작
docker compose down                  # 중지, 데이터 유지
docker compose logs -f tdrive         # 로그
docker compose up -d --build          # 변경 사항 반영
docker compose exec tdrive tdrive help
```

개인 드라이브 암호화 업로드는 최초에 `docker compose exec tdrive tdrive vault create`로 금고를 설정하세요. 재시작 후 잠겨 있으면 `docker compose exec tdrive tdrive unlock`을 실행하세요.

- 웹은 별도 로그인 없이 접근하므로 신뢰하는 Tailscale 사용자에게만 허용하세요.
- 상태는 Docker 볼륨에 저장됩니다. `docker compose down -v`는 볼륨까지 삭제합니다.
- `.env`와 인증 정보는 공유하지 마세요. 암호화 비밀번호를 잊으면 파일을 복구할 수 없습니다. 파일 이름과 크기는 암호화되지 않습니다.
- Tailscale 컨테이너를 재생성했다면 `docker compose up -d --force-recreate tdrive`도 실행하세요.

[라이선스](LICENSE)
