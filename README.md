# TDrive

Telegram에 파일을 저장하고 웹에서 관리하는 드라이브입니다. Docker Compose로 실행합니다.

코드 수정과 테스트는 [개발자 가이드](README.dev.md)를 참고하세요.

## 처음 실행하기

Docker Compose v2, `/dev/net/tun`을 지원하는 환경, Telegram 계정, Tailscale이 필요합니다. 명령은 저장소 최상위에서 실행하세요.

1. [Telegram API ID·hash](https://my.telegram.org/apps)와 [Tailscale 인증 키](https://login.tailscale.com/admin/settings/keys)를 준비하세요.
2. `.env`가 없으면 `cp .env.example .env`로 만들고 `TS_AUTHKEY`를 입력하세요.

```sh
docker compose up -d --build
```

3. Telegram에 로그인하고 사용할 드라이브를 선택하세요.

```sh
docker compose exec tdrive tdrive setup
docker compose exec tdrive tdrive login
docker compose exec tdrive tdrive drives
```

`drives` 출력에 `*`가 없으면 선택하세요.

```sh
docker compose exec tdrive tdrive drive use '드라이브 이름 또는 ID'
```

4. 아래 IP를 확인하고, Tailscale에 연결된 기기에서 `http://<IP>:8080`을 여세요.

```sh
docker compose exec tailscale tailscale ip -4
```

드라이브를 선택하면 웹 서버가 자동으로 시작합니다. 웹에서 업로드·다운로드·검색·폴더 관리·미리 보기를 사용할 수 있습니다.

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
