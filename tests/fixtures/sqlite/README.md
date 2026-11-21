# Phase 04 Go-produced SQLite fixtures

Go v16 synthetic fixture is generated with the real Go schema and migration
routines, not handmade SQL schema. Use this exact command from repository root:

~~~sh
mkdir -p tests/fixtures/sqlite
TDRIVE_COMPAT_FIXTURE_OUT="$PWD/tests/fixtures/sqlite/go-v16.sqlite" \
  go test ./backend/projection -run '^TestExportRustV16Fixture$' -count=1
~~~

The fixture contains a fake personal channel, one folder, one file, and dirents.
It holds no Telegram credentials, real messages or user data. A preexisting
output path is rejected. The DB file is a CI artifact, never committed to Git.

The Rust CI job consumes the fixture, checks schema version 16, counts and
integrity in READ ONLY mode. Neither live user DB nor migrations are touched.
Never copy a live WAL-mode SQLite main file without its WAL. Production
snapshots require the SQLite backup API or coordinated checkpoint/shutdown.
