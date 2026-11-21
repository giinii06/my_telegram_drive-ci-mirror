# Phase 07 — 금고(Vault), TDE1 암호화, 암호화 이름, 보안 상태

**선행** Phase 02·04·06 / **결과** Rust가 기존 암호화 파일과 이름을 정확히 읽고, 쓰기 이전에 권한/잠금/키 수명 정책을 검증.

## 현재 코드와 보안 모델

- backend/crypto/vault.go: Argon2id(password→KEK), XChaCha20-Poly1305 master key wrapping, key check.
- backend/crypto/stream.go: TDE1 magic, 50B header, 64KiB chunk, 16B AEAD tag, HKDF-SHA256 subkey, final chunk authentication.
- backend/crypto/random_access.go: 인증된 byte-range decrypt, 정해진 stored size 검사, final chunk 검증 후 노출.
- backend/crypto/name.go: v1:base64url(nonce||ciphertext), name key version 1, 암호화 파일명에 난수 nonce.
- backend/mountpolicy/policy.go: 원격 policy 미확인 시 plaintext eligibility 거부.
- backend/core/engine.go: vault clearing, generation counter, encrypted media-session publication gate.
- backend/services/encryption/*, projection/encryption*.go: config와 정책 rebuild.
- docker/entrypoint.sh: 기존 vault.pass 자동 잠금 해제 시도(설계 승인 필요).

## 목표 Rust 파일

- crates/tdrive-crypto/src/{vault.rs,kdf.rs,aead.rs,stream.rs,random_access.rs,filename.rs,keyring.rs,test_vectors.rs}
- crates/tdrive-service/src/{vault_service.rs,encryption_policy.rs,session_generation.rs}
- tests/compatibility/{tde1_vectors.rs,vault_material.rs,name_envelope.rs}
- tests/e2e/{lock_race.rs,encrypted_mount.rs,offline_policy.rs}

## 반드시 재현할 실제 바이트 레이아웃

~~~text
TDE1:
  magic:          "TDE1" (4 bytes)
  flags:          1 byte, currently 0
  chunk_log2:     1 byte, v1 = 16 (64 KiB)
  file_salt:      16 bytes
  nonce_prefix:   20 bytes
  plaintext_size: 8-byte little-endian
  total header:   50 bytes
  chunk:          encrypted(<=64KiB) + 16-byte tag
  final:          nonce counter의 final marker를 인증
~~~

Cryptographic details are contract, not redesign. Follow Go deriveSubkey and nonce functions exactly; do not substitute a similar-looking HKDF salt/info or AEAD associated data. Param serialization and Argon2id default/bounds must match stored JSON, including memory measured in KiB.

## 단계별 작업

1. Go fixture generator로 deterministic password/salt/master/subkey/nonce와 TDE1 0/1/65535/65536/65537/131072 bytes 생성. 실제 운영 비밀번호·키는 금지.
2. Rust vault material 파서: Argon2id parameters JSON, limits, KDF type, salt length, wrappedMasterKey length, key check. Wrong password, malformed, oversized payload 모두 안전한 오류.
3. KEK 생성/master wrapping/unwrapping; Go→Rust, Rust→Go 상호복호 테스트. 임의 향상된 default KDF 적용으로 기존 암호 불호환하게 만들지 않는다.
4. TDE1 stream encrypt/decrypt: 정확한 50B header/size 계산, HKDF info, chunk counter, final chunk nonce/auth, extra bytes rejection, final truncate detection, no allocation proportional to huge file.
5. random-access decryptor: offset→ciphertext chunk map, immutable header, expected stored size, clone/read concurrency, close/cancel, last-tag validation BEFORE stream/session publish.
6. filename envelope: v1 prefix, base64 RawURLEncoding semantics, 24-byte random nonce, AEAD AD/value, key version(kid) and unknown format handling. UTF-8 4096B limit 유지.
7. memory safety: zeroize/SecretVec 등 사용 범위를 명시, key copied into structs/UI/log 금지. process core dump / swap and container volume secret policy 검토(보장은 하지 않음).
8. **policy fail-closed**: 현재 개인 채널의 replay에서 policy가 없으면 authoritative sync로 확인한 뒤 plaintext 허용. 네트워크 실패/DB 손상 중에는 평문 쓰기 비활성화.
9. **vault generation**: lock/unlock 중 encrypted media/mount sessions 무효화, 비동기 세션 생성 시 lock generation 재확인. 이전 키로 생성한 token과 cache는 즉시 닫기.
10. Docker vault.pass: 기존 자동 언락 보안과 UX를 ADR-009로 결정. secret file mount/permissions, logs, restart 후 상호작용, user prompt. 기존 설치를 조용히 불능으로 만들지 않는다.
11. 절차가 안정될 때까지 새 버전 암호화 프로토콜 제안은 별도 ADR/Phase로 분리. 이 Phase에서는 TDE1 형식 유지.

## 위협/오류 테스트

- 잘못된 키, 잘못된 KDF params, tampered nonce/header, length overflow, chunk tag 변조, 1B final truncation/extra appended bytes.
- vault 잠금과 동시 range read / thumbnail / mount write / media HLS session open 시 stale plaintext 노출 금지.
- 암호화 policy가 missing vs corrupt vs inaccessible인 세 경우 모두 Go와 동일한 안전한 경계.
- 비밀 데이터가 error, tracing, panic payload, UI store, HTTP JSON, crash logs에 출력되지 않도록 스냅샷/감사.
- 암호화 name의 UTF-8, slash/traversal 및 ciphertext length 제한; 디코딩 실패에 plaintext fallback 금지.

## 완료 조건

- [ ] 100% Go→Rust/Rust→Go TDE1/vault/name golden fixtures
- [ ] 0B·chunk 경계·large file/negative/tamper/final-tag tests
- [ ] cryptographic code independent review, dependencies/unsafe audit
- [ ] vault lock/unlock/session revocation/cancellation concurrency tests
- [ ] offline policy fail-closed, no plaintext bypass
- [ ] data migration 필요 없음; 기존 암호화 파일 bytes 변경하지 않음

**중단 조건**: byte 차이 또는 복호화 실패가 하나라도 있으면 암호화 쓰기 절대 활성화 금지. 이전 암호화 파일 재작성/재암호화 자동 실행 금지.
