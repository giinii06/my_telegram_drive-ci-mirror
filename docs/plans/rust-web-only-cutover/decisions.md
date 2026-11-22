# Decisions — Rust Web-only cutover

| ID | Decision | Source | Status |
|---|---|---|---|
| D-001 | Browser web UI is the only supported interactive client | User, 2026-11-23 | approved |
| D-002 | Android APK/app support is removed, not ported | User, 2026-11-23 | approved |
| D-003 | GPUI/native desktop UI is out of scope | User, 2026-11-23 | approved |
| D-004 | Existing Svelte/Bun frontend remains during backend migration | Repository + web-only scope | approved |
| D-005 | Rust CLI is operator/automation only | Server operation requirement | approved default |
| D-006 | Rust server + Docker/Tailscale is primary runtime | Existing deployment model | approved default |
| D-007 | WebDAV remains; OS mount adapter parity follows actual user-flow inventory | Existing backend surface | deferred; no early deletion |
| D-008 | macOS/Windows packages are headless server installers, not GUI clients | Current packaging source | retain if parity-tested |
| D-009 | Old APK release assets and device-local data are not deleted | Safety boundary | approved |
| D-010 | Go removed only after full parity and rollback proof | Prior Rust-only goal | approved constraint |
| D-011 | No merge, production migration or deployment | Prior explicit boundary | approved constraint |
