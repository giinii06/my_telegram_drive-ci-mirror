# 설치 파일 빌드

`cmd/tdrive` 바이너리 하나를 플랫폼별 설치 파일로 감싼다. 모든 산출물은 저장소 루트의 `dist/`에 쓰이고, `dist/`는 git에 들어가지 않는다.

| 산출물 | 스크립트 | 빌드 호스트 | 세부 문서 |
| --- | --- | --- | --- |
| `tdrive_<버전>_amd64.deb` | `packaging/linux/build-deb.sh` | Linux, macOS | [linux/README.md](linux/README.md) |
| `TDrive-<버전>-universal.dmg` | `packaging/macos/build-dmg.sh` | macOS | [macos/README.md](macos/README.md) |
| `TDrive-Setup-<버전>.exe` | `packaging/windows/build-exe.ps1` | Windows + Inno Setup 6 | [windows/README.md](windows/README.md) |
| `TDrive-Backup-Setup-<버전>.exe` | `packaging/windows/build-backup-exe.ps1` | Windows + Inno Setup 6 | [windows/README.md](windows/README.md) |
| `TDrive-<버전>.apk` | `packaging/android/build-apk.sh` | Linux/macOS + Android SDK + Gradle 8.7 | [android/README.md](android/README.md) |

## 공통 규칙

- 네이티브 설치는 Docker 전용 제한 없이 `tdrive`를 직접 실행한다.
- 모든 설치본은 `TDRIVE_NO_OS_MOUNT=1`을 설정한다. OS 마운트(FUSE·GVfs·WinFsp)는 쓰지 않고 데몬의 내부 WebDAV 엔드포인트로 파일을 스트리밍하므로, 커널 확장이나 데스크톱 서비스 설치가 필요 없다.
- macOS 설치본은 `tdrive web --listen 127.0.0.1`로 루프백에만 열고, `--open`이 기본 브라우저에 토큰이 포함된 URL을 연다. 토큰은 `~/.config/TDrive/web.token`에 있다.
- Windows 서버 설치본은 설치 중 접속 범위를 고른다. `listen.txt`에 `127.0.0.1` 또는 `0.0.0.0`을 쓰고, 포트는 `port.txt`에서 읽는다. 두 파일 모두 설치 폴더에 있다. 두 Windows 설치본 모두 사용자 단위 `HKCU\...\Run` 항목으로 로그인 시 콘솔 없이 시작한다.
- Windows 백업 설치본(`cmd/tdrive-backup`)은 서버의 웹 API만 사용한다. 데몬 소켓이나 새 서버 API가 필요 없다. 업로드는 평문(`encrypt=0`)이라 서버 목록의 바이트 수를 원본과 그대로 비교할 수 있고, 금고가 잠겨 있어도 백업이 멈추지 않는다.
- `--version`을 넘기지 않으면 스크립트가 `git describe --tags --always --dirty`에서 버전을 읽는다.

## 릴리즈 자동화

`.github/workflows/installers.yml`이 `v*` 태그 푸시에서 다섯 산출물을 모두 빌드해 GitHub Release에 첨부한다. `workflow_dispatch`로 실행하면 같은 빌드를 검증만 하고 아티팩트로 남긴다.

- `linux-deb`: `ubuntu-latest`, `dpkg-deb`로 패키징 후 `dpkg-deb -I`로 검사
- `macos-dmg`: `macos-14`, universal 바이너리 빌드 후 dmg를 마운트해 실행 파일 확인
- `windows-exe`: `windows-latest`, `choco install innosetup` 후 PE 헤더 확인
- `windows-backup-exe`: 같은 환경에서 `cmd/tdrive-backup`을 별도로 빌드하고 PE 헤더 확인
- `android-apk`: `ubuntu-latest`, Gradle 8.7 + Android platform 34로 `assembleDebug`, APK 안의 `lib/arm64-v8a/libtdrive.so` 확인

## 알려진 한계

- macOS dmg와 Windows exe는 서명되지 않는다. 첫 실행 시 Gatekeeper 승인 또는 SmartScreen 경고가 나온다.
- Android APK는 디버그 키스토어로 서명한 사이드로드용이다. Play 스토어 배포용 구성이 아니다.
- deb는 amd64만 만든다.
