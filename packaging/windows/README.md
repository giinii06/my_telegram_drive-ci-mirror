# Windows 11 설치 파일 (.exe)

Inno Setup 6으로 만든 사용자 단위 설치 파일입니다. 관리자 권한 없이 현재 Windows 계정에만 설치됩니다.

## 준비물

- Go 1.25 이상 (`go version`으로 확인)
- Inno Setup 6: `winget install JRSoftware.InnoSetup`
  - `ISCC.exe`가 PATH에 없으면 `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`를 사용합니다.

## 빌드

저장소 루트에서 PowerShell을 열고 실행합니다.

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-exe.ps1
```

버전을 직접 지정하려면:

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-exe.ps1 -Version 0.1.0 -Commit $(git rev-parse --short HEAD)
```

결과물은 `dist\TDrive-Setup-<버전>.exe`입니다. 스크립트는 `CGO_ENABLED=0 GOOS=windows GOARCH=amd64`로 `dist\tdrive.exe`를 먼저 빌드하고, `-X main.buildVersion` / `-X main.buildCommit`으로 버전을 심습니다.

## 설치

1. `TDrive-Setup-<버전>.exe`를 실행합니다.
2. 설치 위치는 `%LOCALAPPDATA%\TDrive`이고, 설치 프로그램이 이 폴더를 사용자 PATH에 추가합니다.
3. 원하면 "바탕 화면에 바로 가기 만들기", "Windows를 시작할 때 TDrive 실행"을 선택합니다.
4. 설치 마지막 단계에서 "TDrive 열기"를 체크하면 웹 서버가 시작되고 기본 브라우저가 열립니다.

서명되지 않은 설치 파일이라 SmartScreen 경고가 나옵니다. "추가 정보" → "실행"으로 진행하세요. 코드 서명 인증서가 준비되면 `SignTool`을 Inno Setup에 연결해 경고를 없앨 수 있습니다.

## 사용

- 시작 메뉴의 **TDrive**를 실행하면 `tdrive.exe web --listen 127.0.0.1 --port 8080 --open`이 실행됩니다.
- 콘솔 창이 떠 있는 동안 서버가 동작합니다. 창을 닫으면 서버가 중지됩니다.
- 최초 실행 시 브라우저 온보딩에서 Telegram API 자격 증명과 로그인을 입력합니다.
- 접속 토큰은 `%USERPROFILE%\.config\TDrive\web.token`에 있습니다. `--open`이 토큰이 포함된 주소를 열어 주므로 보통 직접 볼 필요는 없습니다.
- 새 터미널에서 `tdrive version`, `tdrive status` 같은 CLI를 바로 쓸 수 있습니다. PATH가 반영되지 않으면 로그아웃했다가 다시 로그인하세요.

### 포트 변경

`tdrive-web.cmd`는 `TDRIVE_PORT` 환경 변수를 읽습니다. 사용자 환경 변수로 지정합니다.

```powershell
setx TDRIVE_PORT 8090
```

이후 새로 연 TDrive 바로 가기부터 8090 포트를 사용합니다.

### OS 마운트 사용 (선택)

기본 설치는 `TDRIVE_NO_OS_MOUNT=1`로 동작합니다. 즉 Windows 드라이브 문자 마운트를 쓰지 않고, 데몬의 내부 WebDAV 엔드포인트로 웹 UI가 파일을 스트리밍합니다. 드라이브 문자 마운트를 쓰려면:

1. WebClient 서비스를 켜거나 WinFsp를 설치합니다.
2. `tdrive-web.cmd`에서 `set "TDRIVE_NO_OS_MOUNT=1"` 줄을 지우거나 `0`으로 바꿉니다.
3. `tdrive mount start`로 마운트를 시작합니다.

## 제거

설정 → 앱 → 설치된 앱에서 **TDrive**를 제거합니다. 실행 파일과 시작 메뉴 바로 가기가 지워지고 PATH 항목도 함께 제거됩니다. `%LOCALAPPDATA%\TDrive` 아래에 남은 로그와 `%USERPROFILE%\.config\TDrive`의 상태(로그인 세션, 접속 토큰)는 직접 지워야 합니다.

## 검증 체크리스트 (Windows에서 실행)

```powershell
# 1. 빌드 (Inno Setup 설치 후)
powershell -ExecutionPolicy Bypass -File packaging\windows\build-exe.ps1 -Version 0.0.1

# 2. 결과물 확인
Test-Path dist\TDrive-Setup-0.0.1.exe

# 3. 조용한 설치 + 설치 내용 확인
Start-Process dist\TDrive-Setup-0.0.1.exe -ArgumentList "/VERYSILENT","/SUPPRESSMSGBOXES","/NORESTART" -Wait
Test-Path "$env:LOCALAPPDATA\TDrive\tdrive.exe"
(Get-ItemProperty HKCU:\Environment).Path -split ';' | Select-String 'TDrive'

# 4. 서버 기동과 UI 응답
Start-Process "$env:LOCALAPPDATA\TDrive\tdrive-web.cmd"
Invoke-WebRequest http://127.0.0.1:8080/ -UseBasicParsing | Select-Object StatusCode
Get-Content "$env:USERPROFILE\.config\TDrive\web.token"

# 5. 제거 후 PATH 정리 확인
& "$env:LOCALAPPDATA\TDrive\unins000.exe" /VERYSILENT
(Get-ItemProperty HKCU:\Environment).Path -split ';' | Select-String 'TDrive'
```
