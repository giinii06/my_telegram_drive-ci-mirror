# Phase 05 — offline MTProto boundary increment

The Rust Telegram crate defines the object-safe async read/write port, stable
positive MTProto random_id and flood-wait retry budgets, informed directly by
backend/tgclient/client.go and backend/tgclient/idempotency.go.

The old Go client uses a SHA-256-derived stable random ID from a persisted
operation ID and step. The new Rust implementation uses the same domain
separator, UTF-8 bytes, big-endian first eight digest bytes and 63-bit mask.
Go and Rust both consume tests/fixtures/telegram/random-ids.tsv.

This deliberately does NOT use Telegram's Bot API, import gotd session files,
perform login, or connect to Telegram. The MTProto adapter itself (native Rust
or TDLib FFI), cancellation, DC/media streaming, session migration and test
account integration remain open. The write port requires an already-computed
random ID so ambiguous network outcomes cannot silently pick a new one.

Recommended next engineering gate: verify library API and auth state machine on
an isolated user-account sandbox; prove duplicate SendControlWithRandomID and
GetHistory/ChannelDifference parity before enabling remote writes.
