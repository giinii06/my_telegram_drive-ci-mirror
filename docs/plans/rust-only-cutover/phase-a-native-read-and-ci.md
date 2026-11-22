# Phase A — Rust native read-only folder CLI + independent Rust-only CI

**Kind:** user_flow / foundation for API & GPUI. **Status:** implemented (not compiled/tested/verified). **Gate version:** 2.2.

## Entry point
`tdrive-inspect --list-folder <detached-v16-snapshot.sqlite> <positive-channel-id> <parent-id>` reads a previously validated, offline detached Go v16 SQLite snapshot by using `tdrive_storage::list_folder_contents_snapshot`, prints a single JSON object of ordered folder/file records, preserves all precise i64 identifiers as decimal **strings**, exits nonzero on invalid/unsafe snapshot or channel; never mutates SQLite or accesses Telegram/Go daemon. Root parent is empty string.

## Acceptance
- Correct folders/files with escaping of `"\n\t\\` and Korean/Unicode names; stable original Go query order, string-preserved 64-bit IDs.
- Invalid/non-positive/overflowed channel, path missing, source symlink, active WAL sidecar, invalid schema, corrupt DB -> nonzero, no output JSON and no database creation/changes.
- Positive synthetic v16 SQLite fixture and separate CLI integration test run without Go build or daemon. The synthetic schema is a **test fixture** and cannot establish all legacy Go data parity.
- Rust-only CI workflow with exact checked-out SHA, `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build -p tdrive-inspect`. **Do not claim workflow ran** until actual GitHub jobs confirm. Retain existing Go fixture workflow while parity incomplete.
- No new serializer libraries or changed Cargo.lock unless necessary. No direct/raw secret in stdout.

## Implementation
Add a `--list-folder` command to `apps/tdrive-inspect` using a pure Rust formatter module that correctly escapes JSON. Add unit tests and integration fixture using pre-existing `rusqlite` version as a dev-dependency. Add a separate Go-free GitHub workflow that only checks Rust; keep existing CI wiring/visibility untouched. Zero production data impact.

## Blockers
No local cargo available in runtime; GitHub remote Rust job details not yet observed. Phases cannot be called verified/completed without successful compiled CLI and actual tests.

## Latest code and execution checkpoint (2026-11-22)

- **Exact Rust candidate SHA:** `12a39ccd07d79c64a8413083c858b520728c82f8` (prior Rust source `3034b46...`, subsequent unused-import fix at this SHA).
- **Test-only Rust commit:** `beaba9b58ebf01fb182403042f06dafaeaa7861a`: 4 actual-binary tests for SQLite folder list Unicode/escaping/i64, invalid channels and missing files, schema/WAL refusal, symlink/relative-path rejection. **Rust RED not run.**
- **Rust user-flow code:** `3034b46a0d419c860485826a3fdbcd8c129dbfdb` adds `tdrive-inspect --list-folder` and pure Rust JSON generator; `Cargo.toml` and lock file include existing rusqlite 0.43 test dependency; `12a39ccd...` fixes source-order/unneeded `std::path::Path` import. File list is read from detached Go v16 SQLite snapshots, **not** live Telegram or mutable Go database.
- **Go-free Rust-only workflow:** `2530f56945f26a2302fb6f24beabe9c1f2921d23` adds `.github/workflows/rust-only-cutover.yml` with Ubuntu 24.04 and macOS 15 jobs, Cargo fmt/clippy/build/test and offline CLI smoke, **no Go build/test commands**. No mirror settings altered. Existing Go compatibility workflow intentionally still remains.
- **Go removal audit:** `scripts/audit_rust_cutover.py` latest code `8896f29370722925738005f4597cceb9f24e4e4b`. The exact GitLab Python files were copied byte-for-byte locally and independently matched both GitLab SHA-256s: `3a66c3aca0d86c2e04e9897fb76b065d4e446d4ab5e98b8dadde5014fcd5fade` (audit) and `d9ff023795be9dafaa54b01666b04bf40c0b64b472345270f29e8adb3993506a` (tests). `python3 -m unittest scripts/test_audit_rust_cutover.py -v` **5/5 PASS**; `python3 -m py_compile` **PASS**, both actually executed in local temporary read-only copies. This checks only audit logic, not Rust compile.
- **Workflow syntax:** the exact GitLab YAML SHA-256 `8ed563332c4ac7ed4651d5ae3f72d00dfdddb06e773b8c8ea97ae954401f86ba` matched the local copy. PyYAML BaseLoader parsed 2 validly shaped jobs, matrix Ubuntu 24.04/macOS 15, 9+4 steps. This is **not GitHub Actions acceptance or job execution**.
- **Live GitLab repository tree on code SHA `8896f293...`:** still **517** tracked Go files (480 backend, 37 cmd) and `go.mod`/`go.sum`; **37** Rust files. `Go deleted` is **false**. Production Go remains authoritative.
- **Exact SHA GitLab pipeline:** [#3098859551](https://gitlab.com/blond11qq/my_telegram_drive/-/pipelines/3098859551) was requested on code SHA `12a39ccd07d79c64a8413083c858b520728c82f8` and returned GitLab status `success`, `dispatch_github_actions=success`, with zero GitLab Runner jobs. No matching GitHub Rust-only job logs were available at this checkpoint. Classify `dispatch=success`, `provider_result=unknown`, `jobs_verified=unknown`, `e2e_verified=unknown`. Public mirror callback is not Rust-only workflow proof.
- **Commands NOT executed:** `cargo +1.92.0 fmt --all --check`, `cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings`, `cargo +1.92.0 build --workspace --locked`, `cargo +1.92.0 test --workspace --locked`, compiled CLI E2E. No Rust compiler or private GitLab checkout in the container; no secrets/production account.
- **Two review lenses:** [reviews/phase-a.json](reviews/phase-a.json) are distinct sequential self-reviews on exact code SHA; no independent reviewer. Offline diagnostic cannot be represented as complete headless backend. Directory listing is currently unbounded and must be paginated/budgeted before accepting untrusted network inputs. Existing `open_verified_snapshot` and private Go parity remain separate security/integration gates.
- **State:** Phase A `implemented` only, **not tested/verified/completed**. B/C/D/E `not_started`, Go source deletion not executed, no waiver. Stacked Draft MR !24 is open; no merge/deploy/CI provider/mirror/runner/visibility changes.
