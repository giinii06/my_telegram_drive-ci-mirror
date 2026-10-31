# TDrive for macOS

TDrive 서버를 macOS에서 Docker 없이 실행합니다. Intel과 Apple Silicon 모두에서 동작하는 유니버설 빌드입니다.

## 설치

1. `TDrive-<version>-universal.dmg`를 엽니다.
2. **Install TDrive.command**를 더블클릭합니다. 필요하면 Finder에서 우클릭 → **열기**를 선택합니다.
3. 스크립트가 끝나면 기본 브라우저에서 웹 UI가 열립니다.

설치 프로그램은 다음을 만듭니다.

| 경로 | 내용 |
| --- | --- |
| `~/Applications/TDrive/bin/tdrive` | 서버 실행 파일 |
| `~/Library/LaunchAgents/com.tdrive.web.plist` | 로그인 시 자동 실행되는 LaunchAgent |
| `~/Library/Logs/TDrive-web.log` | 서버 로그 |
| `~/.config/TDrive/web.token` | 웹 UI 접근 토큰 (파일 권한 600) |
| `~/Applications/TDrive/bin/Open TDrive.command` | 설치 후 UI를 다시 여는 바로가기 |

서버는 `127.0.0.1:8080`에서만 수신하므로 이 Mac 밖에서는 접속할 수 없습니다. 다른 기기에서 접속하려면 LaunchAgent의 `--listen 127.0.0.1`을 Tailscale 주소로 바꾸거나, Linux `.deb`/Docker 설치를 사용하세요.

## 사용

- UI 열기: `~/Applications/TDrive/bin/Open TDrive.command` 더블클릭
- 서버 상태 확인: `curl -s http://127.0.0.1:8080/api/onboarding/status`
- Telegram 로그인과 드라이브 선택은 웹 UI 온보딩에서 진행합니다.

## 중지 / 시작 / 삭제

```bash
launchctl bootout gui/$UID/com.tdrive.web          # 중지
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.tdrive.web.plist   # 시작
launchctl kickstart -k gui/$UID/com.tdrive.web      # 재시작
rm -rf ~/Applications/TDrive ~/Library/LaunchAgents/com.tdrive.web.plist   # 삭제
```

로그 확인:

```bash
tail -f ~/Library/Logs/TDrive-web.log
```

## 참고

- 이 패키지는 코드 서명이 없습니다. Gatekeeper가 실행을 막으면 Finder에서 우클릭 → **열기**를 한 번 실행하거나 **시스템 설정 → 개인정보 보호 및 보안**에서 허용하세요.
- 동영상 썸네일과 HLS 변환에는 Homebrew의 `mpv`와 `ffmpeg`가 있으면 좋습니다(`brew install mpv ffmpeg`). 설치 프로그램의 PATH에 `/opt/homebrew/bin`이 포함되어 있어 별도 설정은 필요 없습니다.
- 마운트(Finder 볼륨)는 사용하지 않습니다. `TDRIVE_NO_OS_MOUNT=1`로 실행되며 웹 UI는 데몬의 내부 WebDAV 경로를 사용합니다. macFUSE가 필요하지 않습니다.

## 빌드

```bash
packaging/macos/build-dmg.sh --version 1.2.3
```

결과물: `dist/TDrive-1.2.3-universal.dmg`
