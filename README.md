# TDrive

Telegram에 파일을 저장하고 웹에서 관리하는 드라이브입니다. Docker Compose 또는 네이티브 설치 파일(deb · dmg · exe · apk)로 실행합니다.

코드 수정과 테스트는 [개발자 가이드](README.dev.md)를 참고하세요.

## 한 줄 설치

Docker가 설치된 **Linux 서버**에서 실행합니다. Tailscale 인증 키는 [관리 화면](https://login.tailscale.com/admin/settings/keys)에서 발급합니다.

```sh
curl -fsSL https://raw.githubusercontent.com/giinii06/my_telegram_drive/main/install.sh | bash
```

설치 스크립트가 저장소를 `~/.tdrive`에 복제하고 `.env`를 만든 뒤 이미지를 로컬에서 빌드하고, 웹 주소를 출력합니다. Tailscale auth key만 터미널에서 입력하고, 나머지 Telegram 설정은 브라우저에서 진행합니다: API ID·hash, 전화번호, 로그인 코드, 2FA, 드라이브 선택.

`~/.local/bin`에 `tdrive` 명령이 등록됩니다. PATH에 없으면 설치 마지막에 안내가 출력됩니다.

```sh
tdrive onboard   # .env 값 다시 입력
tdrive status
tdrive drives
tdrive help
```

- `tdrive onboard`로 `.env` 값을 다시 입력할 수 있습니다: Tailscale 키, `TDRIVE_PORT`, `TDRIVE_IMAGE`, 그리고 추가 `KEY=VALUE`. Enter만 누르면 기존 값이 유지되고, Tailscale 키가 바뀌면 관련 컨테이너를 다시 만들어 적용합니다.
- `tdrive` 명령은 어느 디렉터리에서든 동작합니다. `docker compose`를 직접 실행할 때는 설치 경로(`~/.tdrive`)로 이동하세요.
- Docker가 없으면 `curl -fsSL https://raw.githubusercontent.com/giinii06/my_telegram_drive/main/install.sh | TDRIVE_INSTALL_DOCKER=1 bash`로 설치를 시도할 수 있습니다.
- 비대화형 환경에서도 설치는 끝나고, 온보딩은 웹에서 진행합니다.
- 온보딩은 Telegram 로그인 전에만 열리며, Tailscale로 연결된 기기에서 접속해야 합니다.
- 로그인을 10분 안에 완료하지 않거나 취소하면 브라우저에서 다시 시도할 수 있습니다.
- 설치 경로는 `TDRIVE_HOME`, 명령 위치는 `TDRIVE_BIN_DIR`로 바꿀 수 있습니다.
- 현재 compose는 `/dev/net/tun`이 필요한 Tailscale 사이드카를 쓰므로 Linux에서만 동작합니다.
- 웹은 Tailscale에 연결된 기기에서 `http://<tailscale-ip>:8080`으로 접근하며 별도 로그인이 없습니다.
- 이미지는 레지스트리에서 내려받지 않고 항상 이 저장소에서 로컬로 빌드합니다. 첫 설치의 빌드는 서버 성능에 따라 몇 분 걸리며, 이후에는 Docker 캐시를 재사용합니다.
- GHCR 이미지나 패키지 공개 설정은 필요하지 않습니다.

수동 설치와 이후 관리 명령은 아래에 정리되어 있습니다.

## 네이티브 설치 파일

Docker 없이 호스트에서 바로 실행하는 설치 파일은 [릴리즈](https://github.com/giinii06/my_telegram_drive/releases)에 첨부됩니다.

| 플랫폼 | 파일 | 설치 |
| --- | --- | --- |
| Linux (amd64) | `tdrive_<버전>_amd64.deb` | `sudo apt install ./tdrive_<버전>_amd64.deb` |
| macOS (Apple Silicon·Intel) | `TDrive-<버전>-universal.dmg` | dmg를 열고 `Install TDrive.command` 실행 |
| Windows 11 | `TDrive-Setup-<버전>.exe` | 설치 후 시작 메뉴의 TDrive |
| Windows 11 자동 백업 | `TDrive-Backup-Setup-<버전>.exe` | 설치 후 서버 주소·백업 폴더 지정 |
| Android | `TDrive-<버전>.apk` | APK 설치 후 앱 실행 |

- 모든 네이티브 설치는 `TDRIVE_NO_OS_MOUNT=1`로 동작합니다. Finder·탐색기·GVfs에 드라이브를 마운트하지 않고 데몬의 내부 WebDAV 엔드포인트로 파일을 스트리밍합니다.
- Telegram 로그인은 각 설치본이 여는 웹 UI에서 진행합니다.
- macOS는 `127.0.0.1:8080`에만 열립니다. Windows는 설치 중 접속 범위를 고릅니다: 이 PC만(`127.0.0.1`) 또는 다른 기기 허용(`0.0.0.0`, 토큰 필요). 선택 값은 설치 폴더의 `listen.txt`·`port.txt`에서 바꿀 수 있습니다. 접속 토큰은 `~/.config/TDrive/web.token`(`%USERPROFILE%\.config\TDrive\web.token`)에 있습니다.
- Windows 설치본은 로그인할 때 백그라운드로 자동 실행됩니다. 시작 메뉴의 **TDrive 서버 시작/중지**로 직접 제어합니다.
- deb는 systemd 서비스로 상시 실행됩니다. 포트와 바인딩 주소는 `/etc/default/tdrive`에서 바꾸고, 접속 URL은 `sudo journalctl -u tdrive | grep 'TDrive web:'`에서 확인합니다.
- Android 앱은 포그라운드 서비스로 기기에서 서버를 직접 실행하고, 같은 기기의 WebView로 접속합니다. 마운트와 mpv 포스터는 지원하지 않습니다.

### Windows 11 자동 백업

`TDrive-Backup-Setup-<버전>.exe`는 이 PC의 폴더를 Tailscale로 연결된 TDrive 서버에 하나씩 옮기는 에이전트를 설치합니다.

1. 서버 역할을 할 PC에 `TDrive-Setup-<버전>.exe`를 설치하고, 접속 범위에서 **다른 기기에서도 접속 허용**을 고릅니다. 두 번째 PC에서 서버의 Tailscale 주소(`tailscale ip -4`)에 접속하려면 이 항목이 필요합니다.
2. 백업할 PC에 `TDrive-Backup-Setup-<버전>.exe`를 실행합니다. 서버 주소는 `my-server`, `my-server.tailnet-xxxx.ts.net`, `100.101.102.103`처럼 포트를 빼고 넣어도 되며 생략 시 8080을 씁니다. 토큰은 서버의 `%USERPROFILE%\.config\TDrive\web.token` 값입니다.
3. 백업 폴더를 고릅니다. 기본값은 `%USERPROFILE%\TDriveBackup`입니다.

- 로그인할 때 백그라운드로 자동 시작됩니다. 시작 메뉴의 **TDrive Backup 시작·중지·상태·로그**로 제어합니다.
- 파일은 오래된 것부터 하나씩 처리됩니다: 업로드 → 서버가 Telegram에 저장 → 서버의 임시 파일 삭제 → 서버 목록에서 같은 크기로 확인 → 원본 삭제 → 다음 파일. 확인 전에는 원본을 지우지 않으므로 실패한 파일은 그대로 남고 다음 주기에 다시 시도합니다.
- 백업은 서버의 금고 암호화를 쓰지 않고 평문으로 저장됩니다. 서버를 재시작해 금고가 잠겨 있어도 백업과 복원은 그대로 동작합니다.
- **Telegram에 저장된 사본은 지우지 않습니다.** 원본을 지운 뒤에도 서버의 `/Backup` 폴더에서 계속 받을 수 있습니다.
- 서버 주소·토큰·폴더·주기는 `%LOCALAPPDATA%\TDriveBackup\backup.json`에서 바꾸고, 시작 메뉴의 **TDrive Backup 중지 → 시작**으로 반영합니다.
- 서버가 꺼져 있으면 로그만 남기고 주기적으로 다시 시도합니다. 한 번만 실행하려면 `tdrive-backup.cmd once`를 씁니다.

빌드 방법과 세부 동작은 [packaging/README.md](packaging/README.md)를 참고하세요.

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
