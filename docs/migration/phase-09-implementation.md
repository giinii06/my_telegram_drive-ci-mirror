# Phase 09 — initial HTTP Range compatibility

The first Rust media crate implements only the HTTP Range header decision
logic from the Go backend/media/server.go parseRangeHeader. It intentionally
supports one byte range, suffix ranges and open-ended ranges, and rejects
multiple ranges. The same 18 edge-case fixtures are checked by the real Go
parser and the new Rust parser in CI, including empty streams, clamping and
signed 64-bit overflow.

Valid partial ranges correspond to HTTP 206, unsupported/unsatisfiable ranges
to HTTP 416 with a Content-Range bytes */size response, and a missing range
to HTTP 200. This is pure logic and does NOT run a server, fetch Telegram
content, establish secure session URLs or stream media.

Entirely outstanding are remote range readers with cancellation, safe
content-length enforcement, session tokens, encrypted vault invalidation,
thumbnails, MP4 seek, Matroska remux, HLS playlists, proxy transcoding,
GPUI integration, authenticated HTTP handlers and media E2E testing.

Go media behavior remains authoritative. This increment is not a complete
Phase 09 migration and is not wired to production playback.
