# Phase 04.c — detached Go replay cursor and SHA-256 verification

Scope: bounded, immutable Go SQLite v16 replay_log page reader in Rust; not a replay mutation engine or live daemon.

- Go source contracts: backend/projection/log.go HashHeader and ProjectFromOpTx, schema.go replay_log columns.
- Go fixture creates synthetic replay messages across two channel IDs, including Unicode percent-encoded header, and computes initial hashes using actual production Go HashHeader.
- Rust reads a fixed channel's messages ascending, exclusive cursor after_msg_id, limit 1 through 500; it refuses invalid cursors, active WAL/SHM, missing/incompatible schema and mismatched SHA-256.
- Rust unit tests check tampered-hash refusal, stable paging order and cross-channel isolation. Go↔Rust comparison uses Go-exported synthetic fixtures and checks that the SQLite file bytes never change.
- sha2 version pinned to 0.10.9 and already present in the lock graph.

Not included: applying the replay operation, projection transaction, live writer, Telegram session, vault name resolution, legacy schema migrations or operational snapshots. The whole Phase 04 is still incomplete. Exact commit SHA CI is required before claiming the increment verified.
