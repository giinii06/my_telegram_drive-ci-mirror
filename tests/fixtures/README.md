# Legacy source contract inventory (Phase 00)

This file captures a **source-derived** baseline for the legacy Go implementation. It is not a record of runtime test execution or a full API schema.

Run from the repository root:

~~~sh
python3 scripts/check-legacy-contracts.py
python3 scripts/check-legacy-contracts.py --print
~~~

The script checks legacy declarations for TDX1 and TDE1 magic, DB schema version, IPC protocol version, TDE1 chunk size, every declared OpType, HTTP route registration, and the existing Docker/Tailscale headless boundary.

When intentional changes alter a contract, update this manifest **only with a reviewed compatibility change** and its Go↔Rust differential test evidence. If the source and manifest diverge, the script fails loudly. No Telegram account, API hash, database, or access token is required.

Additional byte-level fixtures and Rust differential tests belong to Phase 02; this inventory is only the first read-only Phase 00 deliverable.
