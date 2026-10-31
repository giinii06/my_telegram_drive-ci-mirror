# TDrive Android 서버 APK

안드로이드 기기에서 TDrive 서버를 직접 실행하는 APK입니다. 앱이 포그라운드
서비스로 Go 서버(`libtdrive.so`)를 띄우고, 같은 기기의 WebView가
`http://127.0.0.1:8080`에 접속해 기존 웹 UI를 그대로 사용합니다. 온보딩
(Telegram API ID/hash, 로그인 코드, 2FA)도 그 화면에서 진행됩니다.

## 빌드 준비물

- Go 1.25 이상
- JDK 17
- Android SDK: platform 34, build-tools 34 (`ANDROID_HOME` 또는 `ANDROID_SDK_ROOT` 필요)
- Gradle 8.7 (`gradle`이 PATH에 있거나 `GRADLE_BIN=/path/to/gradle`)

Gradle wrapper 파일은 저장소에 넣지 않았습니다. 시스템 Gradle을 사용합니다.

## 빌드

저장소 루트에서:

```sh
bash packaging/android/build-apk.sh
```

결과물은 `dist/TDrive-<version>.apk`입니다. 버전은 `git describe`에서 오고,
`TDRIVE_VERSION`, `TDRIVE_VERSION_NAME`, `TDRIVE_VERSION_CODE`로 덮어쓸 수
있습니다. 기본 ABI는 `arm64-v8a`(실기기) 하나이며,
`TDRIVE_ANDROID_ABIS="arm64-v8a x86_64"`처럼 늘릴 수 있습니다. `x86_64`는
Go가 cgo 외부 링크를 요구하므로 NDK 없이는 빌드되지 않습니다.

산출물은 `assembleDebug`와 디버그 키스토어로 서명됩니다. 사이드로드 전용이고
Play 스토어 업로드용 릴리스 서명이 아닙니다.

## 설치

```sh
adb install -r dist/TDrive-<version>.apk
```

기기에 파일을 복사해 설치할 때는 "알 수 없는 앱 설치"를 허용해야 합니다.

## 실행 구조

- `MainActivity`: WebView와 상태 오버레이. 서비스 시작 후 루프백 포트가 열릴
  때까지 짧게 재시도하고, 실패하면 `server.log` 마지막 줄을 보여 줍니다.
- `TdriveService`: `dataSync` 포그라운드 서비스. `nativeLibraryDir`의
  `libtdrive.so`를 다음 환경으로 실행합니다.
  - `HOME=<filesDir>`, `XDG_RUNTIME_DIR=<filesDir>` (데몬 유닉스 소켓), `TMPDIR=<cacheDir>`
  - `TDRIVE_NO_OS_MOUNT=1` — 안드로이드에는 FUSE/GVfs 마운트가 없어 데몬의
    내부 WebDAV 경로로 스트리밍합니다.
  - 인자: `web --listen 127.0.0.1 --port 8080 --token <32 hex>`
- 액세스 토큰은 `filesDir/web.token`에 보관되고 WebView URL에 포함됩니다.
  루프백만 수신하므로 같은 기기 밖에서는 접속할 수 없습니다.

파일 본문은 텔레그램에 저장되고, 앱 데이터(설정·캐시·토큰·로그)만 기기
앱 전용 저장소에 남습니다.

## 로그와 중지

```sh
adb shell run-as dev.tdrive.server cat files/server.log
adb logcat -s TdriveService
```

- 액티비티를 닫으면(`뒤로가기`로 종료) 서비스가 멈추고 데몬도 정지합니다.
- 설정 > 앱 > TDrive > 강제 중지로도 종료할 수 있습니다.
- 장시간 사용하려면 배터리 최적화 예외로 등록하세요.

## 알려진 제한

- 실행 파일은 `nativeLibraryDir`에 추출되어 그곳에서 exec 됩니다. 일부 기기·정책이
  앱 디렉터리 실행을 막으면 `adb logcat -s TdriveService`에 `EACCES`가 남고 화면에
  시작 실패가 표시됩니다. 그때는 다음과 같이 실행 권한을 확인하세요.

  ```sh
  base="$(adb shell pm path dev.tdrive.server | sed 's/package://; s#/base.apk##' | tr -d '\r')"
  adb shell "ls -l $base/lib/arm64"
  ```

- OS 마운트(폴더처럼 열기)는 지원하지 않습니다.
- `mpv`가 없어 동영상 썸네일은 브라우저가 프레임을 캡처해 서버에 저장합니다.
- WebView는 파일을 직접 저장하지 않으므로 다운로드 링크는 시스템 브라우저로
  넘깁니다(URL에 토큰이 포함되어 바로 내려받힙니다).
- 포트는 `TdriveService.PORT`에 고정되어 있습니다.
