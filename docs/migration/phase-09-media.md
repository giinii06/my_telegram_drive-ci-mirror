# Phase 09 — 파일 range, 이미지/영상 미리보기, fMP4/HLS

**선행** Phase 07~08 / **결과** Rust 미디어 계층이 현재 네트워크 스트리밍을 그대로 재현. 미리보기 때문에 원본 파일을 불필요하게 완전 다운로드하지 않음.

## 원본 코드

- backend/media/{service.go,session.go,server.go,proxy.go,range_reader.go,cache.go,resolver.go,hls.go,mp4_index.go,video_thumbnails.go,image_admission.go,types.go}
- backend/media/remux/{plan.go,matroska.go,stream.go,fmp4.go,hls.go,codecs.go,ac3.go,dts.go,source.go}
- backend/thumbnail/{cache.go,generate.go,local.go,local_apple.*,poster*}
- backend/tgclient/{gotd_range.go,download.go,getfile_limiter.go}
- cmd/tdrive/{web_proxy.go,web_thumbnail.go,web.go}
- cmd/tdrive/webui/tests/{gallery.spec.mjs,drive.spec.mjs}

## 목표 Rust 모듈

- crates/tdrive-media/src/{resolver.rs,range.rs,session.rs,token.rs,cache.rs,http.rs,mime.rs,poster.rs,thumbnail.rs,hls.rs,remux/*}
- crates/tdrive-api/src/media_routes.rs
- tests/fixtures/media/{small-mp4,mkv-multiaudio,damaged-container,range-boundary,...}
- tests/compatibility/{media_http.rs,range_read.rs,hls_playlists.rs}
- tests/e2e/{seek_cancel.rs,vault_lock_media.rs,concurrent_sessions.rs}

## 구현 단계

1. **행동 정의**: Go의 media Resolve/OpenStream/OpenImage/Session semantics, content-type, URL token, range/ETag/Content-Length/Content-Range, status codes, MIME fallback, max image size를 HTTP fixture로 고정.
2. **파일 식별자**: logical ID/revision과 remote content segment reference 분리. 암호화 파일과 multipart에서 plaintext/stored size를 정확히 반영.
3. **bounded remote range**: cancellation, io.ReaderAt 동등성, short read/EOF semantics, 요청 offset overflow, expected server 206/416, 중복 인접 chunk coalescing과 DC별 concurrency.
4. **세션**: capability token은 충분한 entropy, session TTL 및 size limit, 원격/로컬 노출 경로, token revocation. 로그/메트릭에 token/path/파일 내용 기재하지 않는다.
5. **암호화 미디어**: TDE1 final tag 인증 후 session publish, vault generation 변경 시 모든 encrypted media sessions 종료. 오래된 GPUI/웹 URL이 재사용될 수 없음을 테스트.
6. **이미지**: admission control(픽셀 수/bytes/MIME), decoded image memory 폭주 방지, thumbnail cache invalidation, orientation, Apple native/portable decoder 대응.
7. **MP4**: moov/mdat 및 seek index 기반 빠른 첫 프레임, range-only index read, index 미지원/손상 파일에서 safe fallback.
8. **Matroska MKV**: 기존 Go 기능은 재인코딩이 아닌 **on-the-fly Matroska→fragmented MP4/HLS remux**. cues/timestamps/keyframes/audio track 구분, fMP4 init와 m4s, playlist duration honesty, 6초 목표 segment(Go plan.go 기준) 비교.
9. **Audio/codec matrix**: H264/H265/AAC/AC3/DTS 등 현재 remux compatibility와 브라우저/플랫폼 디코더 지원을 나눠 기록. 플레이 불가 codec을 재인코딩 없이 재생된다고 주장하지 않는다.
10. **Proxy transcode**: cmd/tdrive/web.go의 수동 proxy 설정, /api/proxy/transcode, /api/proxy/status, /proxy-hls 등 현재 기능을 보존. 직접 remux와 별개 동작으로 분리.
11. **Cache**: temp cleanup, LRU/TTL/size budgets, immutable generation, stale file revision invalidation, cancellation of expensive thumbnail and stream.
12. **E2E 플레이어**: 첫 재생, seek(초반/중간/끝), 연속 드래그, 오디오 트랙 변경, 2명 동시 시청, 네트워크 끊김/복구, vault lock, 파일 삭제/교체, HTTP request cancellation.
13. **성능**: Go 대비 first-frame latency, random seek latency, range request count, memory/RSS, network overhead를 동일 미디어 fixture로 비교.

## 테스트 세부

| 유형 | 성공 기준 |
|---|---|
| 0 / 1 / last-byte range | 정확한 응답 바이트·상태 코드·길이 |
| encrypted chunk boundary | plaintext hash 동일, 다른 키/잘림 거부 |
| multipart random seek | segmentation 경계 포함 동일 출력 |
| damaged MKV/cues missing | panic 없이 명확한 fallback |
| multi-audio HLS | playlist에 모든 지원 트랙, 전환 정상 |
| stale revision | 옛 content를 새 파일로 오인하지 않음 |
| vault lock while streaming | old token invalid, buffer/키 session 해제 |
| simultaneous seeks | bounded CPU/RSS, no duplicate full download |

## 완료 조건

- [ ] Go HTTP/media response snapshots에 대한 strict contract 통과
- [ ] range, encrypted/multipart streaming, MKV→HLS, thumbnails/tests 통과
- [ ] native GPUI와 browser 모두 새 API 연결 시 기능 회귀 없음
- [ ] 성능 기준선과 codec 지원 매트릭스 명시
- [ ] token·cache·키 해제 보안 검증
- [ ] 시스템 codec 라이브러리/FFmpeg 도입 시 라이선스·Docker image 영향 기록

**롤백**: 새 media routes만 flag off하여 기존 Go endpoint 유지. 코덱 변경/저장 형식 변경은 이 Phase 범위 밖.
