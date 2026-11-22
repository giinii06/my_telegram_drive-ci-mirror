# Phase E — remove Go only when behavior is fully replaced

**Status:** blocked/not_started. **User authorization:** delete Go after replacement; no branch merge/deploy authorized.

Preconditions:
- Complete parity matrix for Go auth+session, Telegram user MTProto, projection 17 ops, encrypted uploads/downloads/backup, live sync, HLS/preview/media, WebDAV/mount, daemon API/web UI/CLI and installers across supported platforms; live test only on isolated test account.
- Run non-destructive Go→Rust database migration on a clone, key/wire compatibility, restart/failure/rollback and performance tests. No production data involved.
- All users of Go packages from Docker, scripts, GitHub Actions, front-end, installers, docs are replaced; no broken binary import paths. Cross-OS CI jobs confirm format, clippy, build, unit, full E2E and full migration rollback.
- Delete backend Go, cmd Go, `go.mod`, `go.sum`, go-build workflows only on the dedicated cutover review branch. Keep neutral golden fixtures so forward compatibility can be tested.
- CI must actually build a Go-free checkout with no running Go binaries. Audit `find . -name '*.go'` and `go.mod` references as a failing gate until zero. If any required capability remains unimplemented, **do not delete** Go source and do not report code complete.
- This phase changes no main branch or live environment. Merge/release still require separate approval.

Current result: preconditions not met (517 Go source/tests and major missing Rust services). No deletions executed.
