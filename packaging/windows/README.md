# Windows 11 설치 파일 (.exe)

Inno Setup 6으로 만든 사용자 단위 설치 파일 두 개입니다. 관리자 권한 없이 현재 Windows 계정에만 설치됩니다.

| 파일 | 스크립트 | 설치 내용 |
| --- | --- | --- |
| `TDrive-Setup-<버전>.exe` | `build-exe.ps1` | TDrive 서버와 로그인 시 백그라운드 자동 시작 |
| `TDrive-Backup-Setup-<버전>.exe` | `build-backup-exe.ps1` | 폴더를 서버로 옮기는 자동 백업 에이전트와 로그인 시 자동 시작 |

## 준비물

- Go 1.25 이상 (`go version`으로 확인)
- Inno Setup 6: `winget install JRSoftware.InnoSetup`
  - `ISCC.exe`가 PATH에 없으면 `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`를 사용합니다.

## 빌드

저장소 루트에서 PowerShell을 열고 실행합니다.

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-exe.ps1
powershell -ExecutionPolicy Bypass -File packaging\windows\build-backup-exe.ps1
```

버전을 직접 지정하려면:

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-exe.ps1 -Version 0.1.0 -Commit $(git rev-parse --short HEAD)
powershell -ExecutionPolicy Bypass -File packaging\windows\build-backup-exe.ps1 -Version 0.1.0
```

결과물은 `dist\TDrive-Setup-<버전>.exe`와 `dist\TDrive-Backup-Setup-<버전>.exe`입니다. 서버 스크립트는 `dist\tdrive.exe`를, 백업 스크립트는 `dist\tdrive-backup.exe`를 같은 `CGO_ENABLED=0 GOOS=windows GOARCH=amd64` 조건으로 먼저 빌드하고 `-X main.buildVersion` / `-X main.buildCommit`으로 버전을 심습니다.

## 서버 설치

1. `TDrive-Setup-<버전>.exe`를 실행합니다.
2. 설치 위치는 `%LOCALAPPDATA%\TDrive`입니다. CLI를 직접 쓸 때는 `"%LOCALAPPDATA%\TDrive\tdrive.exe"`처럼 전체 경로를 지정합니다.
3. 포트 페이지에서 웹 포트를 정합니다. 기본값은 8080이고, 설치 폴더의 `port.txt`에서 나중에 바꿀 수 있습니다.
4. 접속 범위를 고릅니다.
   - **이 PC에서만 접속 (127.0.0.1)**: 브라우저가 같은 PC에서만 붙습니다.
   - **다른 기기에서도 접속 허용 (0.0.0.0)**: 다른 PC의 자동 백업이 Tailscale 주소로 붙을 수 있습니다. 접속에는 항상 토큰이 필요합니다.
5. 시작 옵션에서 **Windows에 로그인할 때 백그라운드로 자동 실행**을 켜 둡니다(기본 체크).
6. 설치가 끝나면 브라우저 온보딩에서 Telegram API 자격 증명과 로그인을 입력합니다. 이 값은 지금 넣거나, 나중에 같은 화면에서 다시 넣을 수 있습니다.

로그인 시 자동 시작은 사용자 로그온 작업(`schtasks /SC ONLOGON`, 작업 이름 `TDrive`)으로 등록되고, `wscript.exe`가 콘솔 없이 `tdrive-web.cmd`를 실행합니다.

- 시작 메뉴의 **TDrive**는 서버를 띄우고 토큰이 포함된 주소를 브라우저로 엽니다.
- **TDrive 서버 시작·중지**로 직접 제어합니다.
- 접속 토큰은 `%USERPROFILE%\.config\TDrive\web.token`에 있습니다.
- `%LOCALAPPDATA%\TDrive\tdrive.log`에 서버 로그가 쌓입니다.

### 포트·접속 범위 변경

`tdrive-web.cmd`는 설치 폴더의 `port.txt`(기본 8080)와 `listen.txt`(기본 `0.0.0.0` 또는 설치 때 고른 `127.0.0.1`)를 읽습니다. 값을 고친 뒤 시작 메뉴에서 **TDrive 서버 중지 → 시작**을 실행합니다.

### OS 마운트 사용 (선택)

기본 설치에서는 드라이브 문자 마운트를 쓰지 않고 데몬의 내부 WebDAV 엔드포인트로 웹 UI가 파일을 스트리밍합니다. 드라이브 문자 마운트를 쓰려면:

1. WebClient 서비스를 켜거나 WinFsp를 설치합니다.
2. `tdrive-web.cmd`에서 `set "TDRIVE_NO_OS_MOUNT=1"` 줄을 지우거나 `0`으로 바꿉니다.
3. `tdrive mount start`로 마운트를 시작합니다.

## 자동 백업 설치

`TDrive-Backup-Setup-<버전>.exe`는 파일이 있는 PC에 설치합니다. 서버는 다른 PC에 있어도 되고 같은 PC여도 됩니다.

1. **TDrive 서버 (Tailscale)** 페이지에 서버 주소를 넣습니다: `my-server`, `my-server.tailnet-xxxx.ts.net`, `100.101.102.103`, `100.101.102.103:9090` 모두 됩니다. 포트를 생략하면 8080입니다.
2. 접속 토큰을 넣습니다. 서버의 `%USERPROFILE%\.config\TDrive\web.token` 값입니다. 비워 두면 서버가 403으로 거부하므로 확인 창이 한 번 뜹니다.
3. **자동 백업 폴더**를 고릅니다. 기존 폴더를 찾아보기로 선택하거나, 없으면 새로 만듭니다. 기본값은 `%USERPROFILE%\TDriveBackup`입니다.
4. 설치가 끝나면 백업이 바로 시작되고, 이후에는 로그인할 때 백그라운드로 자동 실행됩니다.

동작 순서는 파일마다 같습니다: 오래된 파일부터 하나씩 업로드 → 서버가 Telegram에 저장 → 서버의 임시 파일 삭제 → 서버 목록에서 같은 바이트 수로 확인 → 원본 삭제 → 다음 파일. 확인에 실패하면 원본은 남고 다음 주기에 다시 시도합니다. **Telegram에 저장된 사본은 지우지 않습니다.**

- 설정 파일: `%LOCALAPPDATA%\TDriveBackup\backup.json`

  ```json
  {
    "host": "my-server",
    "token": "",
    "folder": "C:\\Users\\me\\TDriveBackup",
    "remote": "/Backup",
    "keep_original": false,
    "interval_seconds": 30,
    "settle_seconds": 10,
    "upload_timeout_seconds": 21600
  }
  ```

  서버 주소·토큰·폴더·주기를 여기서 바꾸고 **TDrive Backup 중지 → 시작**을 실행합니다. `keep_original`을 `true`로 두면 원본을 지우지 않습니다.
- 로그: `%LOCALAPPDATA%\TDriveBackup\backup.log`
- 시작 메뉴: **TDrive Backup 상태**(대기 파일 수 확인), **시작**, **중지**, **설정**, **로그**
- 한 번만 실행: `"%LOCALAPPDATA%\TDriveBackup\tdrive-backup.cmd" once`
- 서버가 꺼져 있으면 로그만 남기고 주기적으로 다시 시도합니다. 중복 실행은 `%LOCALAPPDATA%\TDriveBackup\agent.lock`이 막습니다.

## 제거

설정 → 앱 → 설치된 앱에서 해당 항목을 제거합니다.

- 서버: 실행 파일, 시작 메뉴 바로 가기, 로그온 작업이 지워집니다. 로그와 `%USERPROFILE%\.config\TDrive`의 상태(로그인 세션, 접속 토큰)는 직접 지웁니다.
- 백업: 설치 폴더(`backup.json`, 로그, 저널)와 로그인 자동 실행 항목이 지워집니다. Telegram 사본은 그대로 남습니다.

## 검증 체크리스트 (Windows에서 실행)

```powershell
# 1. 빌드 (Inno Setup 설치 후)
powershell -ExecutionPolicy Bypass -File packaging\windows\build-exe.ps1 -Version 0.0.1
powershell -ExecutionPolicy Bypass -File packaging\windows\build-backup-exe.ps1 -Version 0.0.1

# 2. 결과물 확인
Test-Path dist\TDrive-Setup-0.0.1.exe
Test-Path dist\TDrive-Backup-Setup-0.0.1.exe

# 3. 조용한 설치 + 설치 내용 확인
Start-Process dist\TDrive-Setup-0.0.1.exe -ArgumentList "/VERYSILENT","/SUPPRESSMSGBOXES","/NORESTART","/TASKS=autostart" -Wait
Test-Path "$env:LOCALAPPDATA\TDrive\tdrive.exe"
schtasks /Query /TN TDrive

# 4. 서버 기동과 UI 응답
Start-Process "C:\Windows\System32\wscript.exe" "$env:LOCALAPPDATA\TDrive\tdrive-hidden.vbs"
Invoke-WebRequest http://127.0.0.1:8080/ -UseBasicParsing | Select-Object StatusCode
Get-Content "$env:USERPROFILE\.config\TDrive\web.token"

# 5. 백업 설치와 실행 (서버 주소·폴더는 실제 값으로)
Start-Process dist\TDrive-Backup-Setup-0.0.1.exe -ArgumentList "/VERYSILENT","/SUPPRESSMSGBOXES","/NORESTART" -Wait
Get-Content "$env:LOCALAPPDATA\TDriveBackup\backup.json"
& "$env:LOCALAPPDATA\TDriveBackup\tdrive-backup.cmd" status

# 6. 제거 후 정리 확인
& "$env:LOCALAPPDATA\TDrive\unins000.exe" /VERYSILENT
schtasks /Query /TN TDrive
```

`/VERYSILENT` 설치에서는 사용자 입력 페이지가 기본값으로 채워집니다(포트 8080, 접속 범위 `0.0.0.0`, 백업 폴더 `%USERPROFILE%\TDriveBackup`, 서버 주소 빈 값). 서버 주소가 빈 백업 설치는 실행 즉시 설정 오류를 로그에 남기므로, 조용한 설치 뒤에는 `backup.json`을 실제 값으로 채우고 다시 시작하세요.
