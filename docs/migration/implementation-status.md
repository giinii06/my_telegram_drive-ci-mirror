# Rust/GPUI/Docker migration — implementation status

Baseline: legacy Go main commit 7edfea5456f4e9b18305e15e9de5d149976331da. This implementation branch is based on docs/rust-gpui-docker-migration-phases-20261121 and does **not** update main.

## Phase status

| Phase | Implemented in this branch | Overall complete? |
|---|---|---|
| 00 | Source-only legacy contract checker for schema v16, daemon protocol v6, TDX1/TDE1 identifiers, 17 ops, 25 HTTP routes, Docker/Tailscale invariant | **No**: real Go full tests/performance and runtime baseline outstanding |
| 01 | Detailed architecture, compatibility matrix and ADR documents inherited from docs MR | **No**: key ADRs require spikes and approvals |
| 02 | 43 curated TDX1 positive/negative cases; standalone Rust tests and Go oracle regression test | **No**: cryptographic, DB, media, HTTP/IPC golden fixtures outstanding |
| 03 | Cargo workspace, 64-bit-safe positive IDs, isolated tdrive-inspect executable and GitLab CI jobs | **No**: cargo compiler/job execution not yet confirmed |
| 04 | None: SQLite schema migration and read-only DB parity still Go-owned | **Not started** |
| 05 | None: authenticated Telegram/MTProto remains Go-owned | **Not started** |
| 06 | Read-only Rust parser that recognizes all 17 TDX1 operation kinds with required-field validation | **No**: canonical formatter, positive rendition, projection, replay, SQLite and sync not ported |
| 07 | Read-only TDE1 50-byte header/size inspector with ciphertext-size bounds tests | **No**: no key derivation, crypto, stream decrypt or vault state |
| 08–15 | No production Rust components; Go keeps file transfer, media, mount, server, web, packaging and mobile | **Not started** |

**Safety**: The Rust code currently has NO database writer, no Telegram network adapter, no vault key handling, no GPUI, and no Rust Docker production image. It cannot be run as a replacement for Go. The existing Go binaries, DB, .env, Docker image/Compose, client, and package scripts remain unchanged.

## Known limits

- TDX1 rendition envelopes (rend=...) are rejected pending typed JSON descriptor validation; Go can accept valid renditions. This is a deliberately visible compatibility gap.
- The Rust UTF-8 path rejects invalid UTF-8; Go's QueryUnescape returns raw byte strings. Production parity must decide how to represent these bytes without loss.
- The legacy source inventory guards declarations and route names only; it is not full semantic parity.
- The TDE1 inspector checks magic, flags, salt/nonce offsets and expected file size. It **does not authenticate or decrypt data**; it cannot be used for access decisions.
- No actual build/test success has been observed for Rust at the commit where this document was written. Full CI remains a gate, not a formality.

## CI configuration and operator dependency

The project has an Oracle A1 ARM64 runner #59776971, tag oracle-arm64, associated with Pmanager. In the GitLab project Settings → CI/CD → Runners, it must be explicitly assigned to my_telegram_drive and potentially unlocked first. The CI branch targets the tag, but an unassigned runner leaves all jobs pending. See [runner setup](ci-runner-setup.md).

Planned verification commands:

~~~sh
python3 scripts/check-legacy-contracts.py
go test ./backend/projection -run TestRustTDX1Fixtures -count=1
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
printf 'TDX1|t=f|p=|n=hello.txt\n' | cargo run --locked -p tdrive-inspect -- --check-stdin
~~~

Do not claim a Phase fully complete without test execution and the downstream Phase acceptance gates. Do not merge/deploy this branch until a reviewer has inspected the result and CI has run at its exact SHA.

## Next verified changes required

1. Runner assignment, CI execution; repair any demonstrated Rust/Go test failures and record logs.
2. TDX1 descriptor/rendition support plus canonical Format in Rust; compare parsed field values, not only operation kinds.
3. Go→Rust SQLite v16 snapshot read, schema table/index inventory, replay table diff.
4. Telegram user-account MTProto stack spike (safe test account; no two processes writing same channel).
5. TDE1 Go↔Rust actual cryptographic golden vectors, failure tests and key lifecycle.
6. Only then begin real file services, API/CLI, GPUI desktop and Rust Docker headless cutover in independent MRs.
