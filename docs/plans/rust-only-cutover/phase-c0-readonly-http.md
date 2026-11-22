# Phase C.0 — authenticated loopback Rust HTTP snapshot API

**Purpose:** Go-free API entry point for the existing offline Rust folder listing. This is a **strictly read-only, local development diagnostic** and eventual GPUI/web service foundation. It is NOT MTProto auth, uploads/downloads, encrypted media, remote API, Docker production daemon or Go parity.

## Interface and security
- New Unix-only command: `tdrive-inspect --serve-readonly <absolute-detached-v16-snapshot> <absolute-private-token-file> <port>`.
- Listen **only** on `127.0.0.1`, never 0.0.0.0. Reject port 0 to avoid accidental auto-binding. Prevalidate detached SQLite snapshot with existing Rust v16 snapshot checker. Must not open a production database or write it.
- Require token file to be a regular final non-symlink file, owner-only Unix mode 0600 or stricter, 32–128 ASCII `A-Za-z0-9_- ` token bytes, no newline; pass only the path as argument. Refuse insecure token.
- Close every HTTP/1.1 request after response. Accept exactly one GET request and bounded 8 KiB headers per connection, short I/O timeouts; no request body, no keepalive, no public endpoint. Reject missing/multiple/malformed Authorization, unsupported verbs and excessively long headers. Use constant-time comparison of provided bearer bytes. `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`.
- Authenticated `GET /health` → `{"status":"ok"}`. Authenticated `GET /v1/folders?channel=<positive-int64>&parent=<urlencoded-folder-id>` → same JSON as existing `--list-folder`; wrong/unsafe inputs must not reveal SQLite content.
- Tests for authorization and malformed/multiple credentials, bounds and route parsing. Full compiled socket integration and macOS/Ubuntu CI jobs required before marking tested.

## Restrictions
No TLS/HTTPS, multi-user, public bind, write endpoints, data deletion, logs containing token, live Go backend, Telegram access, GPUI, Docker release. Do not deploy. Existing Phase C still unverified; this design is only C.0. Missing Rust compiler means source may be committed as **unverified** after static review; neither current GitLab CI bridge nor its success callback validates binary behavior.
