# Implementation progress (living status)

The previous Phase 00–15 documents describe the full Rust/GPUI/Docker target. This file reports **implemented and verified work**, not ambitions.

| Phase | Actual implemented increment | Completion state |
|---|---|---|
| 00 | Source-derived contract inventory (schema=16, daemon=6, TDX1/TDE1, operations, HTTP routes, Docker sidecar) | Partial; no full runtime baselines |
| 01 | Architecture/ADR documents on predecessor docs branch | Design written, ADR decisions still open |
| 02 | 43 curated TDX1 examples + matching Go and Rust tests | Partial; full crypto/DB/API fixture collection outstanding |
| 03 | Cargo workspace, IDs, offline inspect app, .gitlab-ci.yml | Partial; compile/CI runner results outstanding |
| 04 | None; legacy SQLite v16 still exclusively Go-owned | Not started |
| 05 | None; MTProto and Telegram session still exclusively Go-owned | Not started |
| 06 | Read-only TDX1 Rust parser, all 17 operation kinds recognized | Partial; rendition JSON, canonical format, projection and sync not ported |
| 07–15 | No production Rust implementation | Not started |

## Safety boundary

- The Go daemon / original Docker / Svelte web / installer remain the functioning product.
- Rust TDX1 validation is a *non-production* offline component; it is not wired to database or Telegram writes.
- TDX1 rendition envelopes are rejected until their descriptor validation is implemented, so this decoder is not Go-compatible for all valid headers.
- Go can preserve arbitrary invalid UTF-8 byte sequences in strings; Rust's String decoder currently fails closed on invalid UTF-8.
- Do not use Rust parser to replace production Go until differential tests cover all valid headers and exact error/value equivalence.
- Compilation, benchmark, CI status, and actual integration results must be reported with matching SHA and job URLs.
- The GitLab project reports shared runners disabled. CI job creation/pending does not demonstrate execution.

## How to verify

~~~sh
python3 scripts/check-legacy-contracts.py
go test ./backend/projection -run TestRustTDX1Fixtures -count=1
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
printf '%s\n' 'TDX1|t=f|p=|n=hello.txt' | cargo run --locked -p tdrive-inspect -- --check-stdin
~~~

**Further incremental work**: 1) obtain actual CI execution; 2) expand TDX1 positive rendition and encrypted metadata roundtrips; 3) SQLite v16 read-only snapshot and replay table diff; 4) MTProto engine spike with an isolated test account; 5) only then replace any production path.

The work should proceed as separate reviewable commits or MRs and must not merge/deploy automatically.
