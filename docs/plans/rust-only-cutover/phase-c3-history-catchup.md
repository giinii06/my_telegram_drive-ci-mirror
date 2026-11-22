# Phase C.3 — Crash-consistent Telegram history catch-up (still not full production sync)

**Status:** planned; part of existing Phase C (`in_progress`, unverified). Committed before code. User outcome: after an authenticated Rust user client is restarted, `tdrive-mtproto catchup PEER BUDGET` scans Telegram history newest→oldest down to its **previously committed** highest message ID, appends only parsed TDX1 entries to the separate private inbox, and persists the new high-watermark *only after* scan reaches the old watermark or exhausts history. Missing interval, network error, unparsable TDX1, or budget exhaustion must leave old watermark unchanged so a subsequent run retries safely.

## Approved scope and architecture
GitLab project 89276294; Draft MR !24; branch `refactor/rust-only-cutover-20261122` code baseline `1cddb6e97621ee21a3c0530241fba90057dc6dd0`. Go service and Go v16 SQLite still authoritative. No merge, deploy, user credentials, runner/mirror/visibility setting changes. Existing C.0–C.2 are source-only/unverified; source CI #3098911766 has **failed** external callback, no remote job log. There is no Rust compiler, Docker or private GitLab checkout in the current execution container. No passing test claim is available. This is an implementation attempt, not permission to waive CI.

### Data boundaries
- Keep native `tdrive-mtproto` nested workspace; new `native_inbox_progress(peer_key PRIMARY KEY, max_seen_msg_id)` additive table alongside existing `native_inbox_events`, `native_inbox_tamper`. Schema v1 metadata must support legacy databases created by C.2: a writable open performs `CREATE TABLE IF NOT EXISTS` only on this *native inbox*; never touches Go production DB. A read-only open cannot migrate.
- Use existing `InboxEvent` and `Inbox::append` for idempotent first-seen/edited message audit. Cursor commits are independent of the record writes, but must occur strictly **after** complete interval scan. If interrupted, already-recorded entries repeat safely. Database failure or Telegram error leaves cursor unchanged.
- Only count/advance positions on history messages with ID greater than the previous cursor; older/equal IDs stop scan. First scan without cursor requires exhaustive history (or fails with `BudgetExhausted`), never silently initializes to last 100.
- Upper bound `budget` 1–10_000 network messages per run; request `budget+1` where supported to detect truncation, and if the extra message is still newer than old cursor, abort without cursor advance. Write TDX1 header records only; skip unrelated; invalid TDX1 aborts. The first seen max ID is persisted as new watermark only on complete scan. A cursor update must be monotonic and no-clobber even under competing caller; serialize with IMMEDIATE transaction.
- `catchup` is one bounded run; no long-running background subscription and no Telegram getChannelDifference, edits older than committed watermark or deletes. Neither GPUI nor full 17-op Go projection nor vault/media/WebDAV are done by this phase; these are separate essential product gates.

### Test-first outcome / negative cases
1. New private DB and existing v1 DB open; progress absent initially, persists monotonic positions across restart; empty/zero/negative or wrong peer rejected. No writes to Go v16 file.
2. Simulate scan progress: first complete run can advance; interrupted run or too many messages returns explicit failure without advancement; replay of same records no duplicate rows; next success advances. Regress/backward cursor rejected; overlapping concurrent writers cannot lower cursor.
3. Existing TDX1 malformed message aborts (never silently skip); non-TDX1 messages skip; source errors/flood wait propagate; no new secret output.
4. Actual `cargo fmt --check`, clippy -D warnings, `cargo test`, compiled `catchup` with an isolated Telegram test account and positive/restart/long-gap tests, exact source SHA GitHub jobs required. No local Cargo means no TDD RED/GREEN verification, and plan status remains blocked/unverified.

### Full user-flow consumer
C.4 will consume the inbox+cursor into the complete 17-op Go-compatible projection, then authenticated vault, media/WebDAV and headless API, GPUI, migrations/rollback and final Go deletion. A history high-watermark only covers newly created messages, **not** all Telegram updates. Do not call Phase C.3 a full production sync engine.
