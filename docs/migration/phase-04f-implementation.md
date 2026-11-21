# Phase 04.f — Go/Rust backend.lock compatibility (conservative increment)

The legacy Go processlock.Acquire uses O_CREATE|O_EXCL on `<Go config dir>/TDrive/backend.lock` and writes JSON role/pid/started_at. Rust now uses the same path, atomic exclusive create and JSON field names. A held Go lock is **never** reclaimed by Rust, regardless of apparent PID liveness. A held Rust lock prevents Go's normal second-process acquisition (the Rust process PID is live under the same PID namespace).

Safety properties:
- The ConfigDirs root must already pass private-directory preflight. Locks are created 0600 and file+directory synced before signaling success.
- A second Rust contender fails with LockHeld; concurrent 12-thread test proves only one holder. Existing manually created Go-format file remains untouched.
- The Rust guard verifies device+inode before unlink on release, and refuses to remove a replaced lock. Explicit release and Drop both attempt cleanup.
- Untrusted role strings that could inject JSON are rejected.
- Windows lock acquisition deliberately unsupported until a same-volume create-new and Windows ACL implementation is proven.

**Not Phase 04.f complete**: legacy Go permits stale lock cleanup on a dead PID; Rust conservatively refuses automatic stale reclamation. PID namespaces and shared Docker host volumes can still make legacy Go incorrectly reclaim a live Rust lock under a different namespace. Production safety therefore additionally requires a shared advisory/SQLite writer lock enforced on **both** Go and Rust paths, with cross-container and crash tests. The Rust guard is not wired into a server and must not be used to claim a safe live cutover.

Verification: cargo fmt/clippy/test + original Go processlock package tests in GitLab and GitHub. No production lock or user data touched.
