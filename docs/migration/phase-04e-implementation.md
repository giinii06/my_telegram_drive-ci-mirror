# Phase 04.e — Rust config paths and secret file safety (Unix incremental)

Authoritative Go references:
- backend/datadir/datadir.go: Config/Cache base override semantics, /TDrive suffix, private 0700 directories.
- backend/auth/{auth.go,config.go,private_file.go,session_storage.go}: imp_config.json/config.json/session.json, atomic secret 0600 rename and parent directory sync.
- Go backend/processlock and SQLite remain sole operational writer.

This subphase adds crates/tdrive-config, a self-contained Rust library:
- explicit data/cache base directories plus platform user defaults, with separate /TDrive folders
- fails closed on absolute/clean-path violation, symlink/non-directory ancestors, app directory not 0700/private, and private file symlinks
- typed filenames prevent arbitrary path injection; missing session returns None, empty session is rejected
- 0600 create_new same-directory temporary file, write_all, file sync, atomic rename, directory sync on Unix
- rejects too-large payload (>1 MiB) and overly broad permissions on existing secret files; sensitive values are never logged
- Linux and macOS first; Windows ACL-secure writes are **unsupported** rather than pretending Unix mode bits secure Windows
- deterministic tests cover overrides, missing/replace, symlinks, external-file preservation, file modes, oversize and invalid paths

Security restriction: current path preflight checks and final rename run under a protected 0700 application folder, but untrusted concurrently mutable ancestor directories can still cause TOCTOU. A directory-descriptor openat/openat2 design and real owner/ACL validation remain Phase 04.e hardening prerequisites before production release. This code is not yet wired to the Go session or a Rust Telegram runtime, and it never reads or modifies live user's config in CI.

The wider Phase 04 remains incomplete: config JSON schema parsing, Windows ACLs, process and advisory locks, WAL-aware backup, migration rollback and transactional writer await dedicated testable increments.
