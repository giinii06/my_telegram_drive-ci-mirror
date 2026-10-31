# Linux .deb 설치

TDrive 서버를 우분투/데비안에 네이티브로 설치합니다. Docker 없이 동작합니다.

## 빌드

```bash
bash packaging/linux/build-deb.sh [--version 1.2.3]
```

결과: `dist/tdrive_<version>_amd64.deb`

`dpkg-deb`가 있으면 그것으로, 없으면(macOS 등) 동일한 구조의 ar 아카이브를 직접 만들어냅니다. 어느 쪽이든 `ar t`로 `debian-binary`, `control.tar.gz`, `data.tar.gz`가 확인되어야 합니다.

## 설치

```bash
sudo apt install ./tdrive_<version>_amd64.deb
```

패키지가 하는 일:

- `/usr/bin/tdrive` 설치
- `tdrive` 시스템 사용자 생성(`/var/lib/tdrive` 홈)
- `tdrive.service` 활성화·시작
- `/etc/default/tdrive`로 포트·바인드 주소 설정

## 최초 설정

Telegram 로그인은 서비스 사용자로 실행합니다.

```bash
sudo -u tdrive tdrive setup
sudo -u tdrive tdrive login
sudo -u tdrive tdrive drives
sudo -u tdrive tdrive drive use <이름 또는 ID>
```

로그인 후 웹 UI 주소를 확인합니다. 접속 토큰이 URL에 포함되어 있으니 그대로 사용합니다.

```bash
sudo journalctl -u tdrive | grep 'TDrive web:'
sudo cat /var/lib/tdrive/.config/TDrive/web.token   # 토큰만 필요할 때
```

주소는 `http://<호스트>:8080/?token=...` 형태입니다.

## 설정

`/etc/default/tdrive`:

| 값 | 기본값 | 설명 |
| --- | --- | --- |
| `TDRIVE_LISTEN` | `0.0.0.0` | 바인드 주소. Tailscale 주소로 바꾸면 tailnet 전용이 됩니다. |
| `TDRIVE_PORT` | `8080` | 웹 포트 |
| `TDRIVE_NO_OS_MOUNT` | `1` | `0`이면 OS 마운트(GVfs)를 사용합니다. |

변경 후:

```bash
sudo systemctl restart tdrive
sudo systemctl status tdrive
sudo journalctl -u tdrive -f
```

## 제거

```bash
sudo apt remove tdrive        # 설정과 데이터 유지
sudo rm -rf /var/lib/tdrive   # 데이터까지 삭제
```

## 참고

- 마운트는 기본으로 꺼져 있습니다. 파일 전송은 데몬의 사설 WebDAV 엔드포인트를 거치므로 데스크톱 세션이 없어도 웹 UI와 스트리밍이 동작합니다.
- 동영상 썸네일과 프록시 트랜스코드는 `mpv`, `ffmpeg`가 있으면 켜집니다(`Recommends`). 서버 용도라면 직접 설치하세요: `sudo apt install ffmpeg mpv`.
- 토큰은 `--no-auth` 없이 실행할 때 자동 생성됩니다. 서비스 파일은 이 기본 동작을 사용합니다.
