package projection

import (
    "database/sql"
    "crypto/sha256"
    "encoding/binary"
    "hash"
    "fmt"
    "strings"
    "os"
    "path/filepath"
    "testing"

    _ "modernc.org/sqlite"
)

// TestExportRustV16Fixture exports a new synthetic database using the actual
// Go schema and migrations. Explicit output path is required; never user data.
func TestExportRustV16Fixture(t *testing.T) {
    destination := os.Getenv("TDRIVE_COMPAT_FIXTURE_OUT")
    if destination == "" {
        t.Skip("TDRIVE_COMPAT_FIXTURE_OUT not specified")
    }
    if !filepath.IsAbs(destination) {
        t.Fatal("fixture output path must be absolute")
    }
    if err := os.MkdirAll(filepath.Dir(destination), 0o700); err != nil {
        t.Fatal(err)
    }
    if _, err := os.Lstat(destination); err == nil {
        t.Fatal("refusing to replace an existing database")
    } else if !os.IsNotExist(err) {
        t.Fatal(err)
    }

    db, err := sql.Open("sqlite", destination)
    if err != nil {
        t.Fatal(err)
    }
    defer db.Close()
    db.SetMaxOpenConns(1)

    if err := EnsureSchema(db); err != nil {
        t.Fatalf("EnsureSchema: %v", err)
    }
    const channelID int64 = 123456
    if err := MigratePersonalChannel(db, channelID); err != nil {
        t.Fatalf("MigratePersonalChannel: %v", err)
    }
    statements := []string{
        "INSERT INTO folders(channel_id,id,name,parent_id,tombstoned,revision) VALUES (123456,'d:docs','Docs','',0,1)",
        "INSERT INTO files(channel_id,msg_id,name,size,parent_id,upload_time,content_msg_id,revision) VALUES (123456,31,'hello.txt',123,'d:docs',1700000000,31,1)",
        "INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned) VALUES (123456,'d:docs','folder','','Docs','docs',1,0)",
        "INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned) VALUES (123456,'f:31','file','d:docs','hello.txt','hello.txt',1,0)",
        "INSERT INTO file_revisions(channel_id,file_msg_id,revision,content_msg_id,committed_msg_id) VALUES (123456,31,1,31,31)",
        "INSERT INTO folders(channel_id,id,name,parent_id,tombstoned,revision) VALUES (123456,'d:archive','Archive','d:docs',1,2)",
        "INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned) VALUES (123456,'d:archive','folder','d:docs','Archive','archive',2,1)",
        "INSERT INTO files(channel_id,msg_id,name,size,parent_id,upload_time,uploader_user_id,tombstoned,encrypted,plaintext_size,content_msg_id,content_hash,revision) VALUES (123456,33,'resume.txt',300,'d:docs',1700000044,77,0,1,250,333,'sha256-test',2)",
        "INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned) VALUES (123456,'f:33','file','d:docs','résumé.txt','résumé.txt',2,0)",
        "INSERT INTO files(channel_id,msg_id,name,size,parent_id,upload_time,tombstoned,content_msg_id) VALUES (123456,34,'gone.txt',25,'d:docs',1700000055,1,34)",
        "INSERT INTO dirents(channel_id,object_id,object_kind,parent_id,display_name,name_key,revision,tombstoned) VALUES (123456,'f:34','file','d:docs','gone.txt','gone.txt',1,1)",
    }
    tx, err := db.Begin()
    if err != nil {
        t.Fatal(err)
    }
    for _, query := range statements {
        if _, err := tx.Exec(query); err != nil {
            _ = tx.Rollback()
            t.Fatalf("fixture insert: %v", err)
        }
    }
    if err := tx.Commit(); err != nil {
        t.Fatal(err)
    }
    // These rows simulate actual received operations using the same
    // HashHeader implementation that Go ProjectFromOp uses.
    syntheticReplay := []struct {
        channel int64
        msg int64
        kind string
        raw string
        actor int64
    }{
        {123456, 101, "mkdir", "TDX1|t=mkdir|obj=d:docs|p=|n=Docs", 77},
        {123456, 103, "f", "TDX1|t=f|p=d:docs|n=%ED%95%9C%EA%B8%80.txt|sz=9", 99},
        {123456, 205, "tomb", "TDX1|t=tomb|obj=f:31", 5},
        {654321, 7, "move", "TDX1|t=move|obj=f:4|p=", 6},
    }
    for _, event := range syntheticReplay {
        _, err := db.Exec(
            "INSERT INTO replay_log(channel_id,msg_id,op_type,op_payload_json,raw_header,first_seen_hash,actor_user_id,seen_at) VALUES (?,?,?,?,?,?,?,?)",
            event.channel, event.msg, event.kind, "{}", event.raw, HashHeader(event.raw), event.actor, int64(1700000000),
        )
        if err != nil {
            t.Fatalf("insert synthetic replay row: %v", err)
        }
    }
    if oraclePath := os.Getenv("TDRIVE_COMPAT_REPLAY_OUT"); oraclePath != "" {
        if !filepath.IsAbs(oraclePath) {
            t.Fatal("replay oracle path must be absolute")
        }
        if err := os.MkdirAll(filepath.Dir(oraclePath), 0o700); err != nil {
            t.Fatal(err)
        }
        if _, err := os.Lstat(oraclePath); err == nil {
            t.Fatal("refusing to overwrite replay oracle")
        } else if !os.IsNotExist(err) {
            t.Fatal(err)
        }
        var oracle strings.Builder
        cases := []struct {
            channel int64
            after int64
            limit int64
        }{{123456, 0, 2}, {123456, 101, 2}, {123456, 103, 2}, {654321, 0, 2}, {654321, 7, 2}}
        for _, c := range cases {
            fmt.Fprintf(&oracle, "CASE\t%d\t%d\t%d\n", c.channel, c.after, c.limit)
            rows, err := db.Query(
                "SELECT channel_id,msg_id,op_type,op_payload_json,raw_header,first_seen_hash,actor_user_id,seen_at FROM replay_log WHERE channel_id=? AND msg_id>? ORDER BY msg_id ASC LIMIT ?",
                c.channel, c.after, c.limit,
            )
            if err != nil {
                t.Fatalf("Go replay oracle select: %v", err)
            }
            for rows.Next() {
                var rowChannel, rowID, actor, seen int64
                var kind, payload, header, hash string
                if err := rows.Scan(&rowChannel, &rowID, &kind, &payload, &header, &hash, &actor, &seen); err != nil {
                    _ = rows.Close()
                    t.Fatal(err)
                }
                if HashHeader(header) != hash {
                    _ = rows.Close()
                    t.Fatal("invalid Go replay hash fixture")
                }
                fmt.Fprintf(&oracle, "OP\t%d\t%d\t%s\t%s\t%s\t%s\t%d\t%d\n",
                    rowChannel, rowID, kind, payload, header, hash, actor, seen)
            }
            if err := rows.Err(); err != nil {
                _ = rows.Close()
                t.Fatal(err)
            }
            if err := rows.Close(); err != nil {
                t.Fatal(err)
            }
        }
        if err := os.WriteFile(oraclePath, []byte(oracle.String()), 0o600); err != nil {
            t.Fatal(err)
        }
    }
    if oraclePath := os.Getenv("TDRIVE_COMPAT_DIGEST_OUT"); oraclePath != "" {
        if !filepath.IsAbs(oraclePath) {
            t.Fatal("digest oracle path must be absolute")
        }
        if err := os.MkdirAll(filepath.Dir(oraclePath), 0o700); err != nil {
            t.Fatal(err)
        }
        if _, err := os.Lstat(oraclePath); err == nil {
            t.Fatal("refusing to replace an existing Go digest oracle")
        } else if !os.IsNotExist(err) {
            t.Fatal(err)
        }
        var oracle strings.Builder
        tables := []string{
            "schema_version", "channels", "replay_log", "replay_log_tamper",
            "replay_log_rejects", "folders", "files", "dirents",
            "file_revisions", "projection_operations", "encryption",
            "file_parts", "pending_joins", "trash_entries",
        }
        for _, table := range tables {
            columnRows, err := db.Query("PRAGMA table_info(\"" + table + "\")")
            if err != nil {
                t.Fatal(err)
            }
            var columns []string
            for columnRows.Next() {
                var cid, notNull, pk int64
                var name, kind string
                var defaultValue sql.NullString
                if err := columnRows.Scan(&cid, &name, &kind, &notNull, &defaultValue, &pk); err != nil {
                    _ = columnRows.Close()
                    t.Fatal(err)
                }
                columns = append(columns, name)
            }
            if err := columnRows.Err(); err != nil {
                _ = columnRows.Close()
                t.Fatal(err)
            }
            _ = columnRows.Close()
            if len(columns) == 0 {
                t.Fatalf("no columns for table %s", table)
            }

            h := sha256.New()
            goDigestFeed(h, table)
            var expressions []string
            for _, column := range columns {
                goDigestFeed(h, column)
                quoted := "\"" + strings.ReplaceAll(column, "\"", "\"\"") + "\""
                expressions = append(expressions, "typeof("+quoted+")", "hex(CAST("+quoted+" AS BLOB))")
            }
            order := make([]string, len(expressions))
            for i := range expressions {
                order[i] = fmt.Sprint(i + 1)
            }
            query := fmt.Sprintf("SELECT %s FROM \"%s\" ORDER BY %s",
                strings.Join(expressions, ","), table, strings.Join(order, ","))
            rows, err := db.Query(query)
            if err != nil {
                t.Fatal(err)
            }
            values := make([]string, len(expressions))
            dest := make([]any, len(expressions))
            for i := range values {
                dest[i] = &values[i]
            }
            count := 0
            for rows.Next() {
                if err := rows.Scan(dest...); err != nil {
                    _ = rows.Close()
                    t.Fatal(err)
                }
                count++
                for _, value := range values {
                    goDigestFeed(h, value)
                }
            }
            if err := rows.Err(); err != nil {
                _ = rows.Close()
                t.Fatal(err)
            }
            if err := rows.Close(); err != nil {
                t.Fatal(err)
            }
            fmt.Fprintf(&oracle, "%s\t%d\t%x\n", table, count, h.Sum(nil))
        }
        if err := os.WriteFile(oraclePath, []byte(oracle.String()), 0o600); err != nil {
            t.Fatal(err)
        }
    }
    var version int
    if err := db.QueryRow("SELECT MAX(version) FROM schema_version").Scan(&version); err != nil {
        t.Fatal(err)
    }
    if version != currentSchemaVersion {
        t.Fatalf("schema version %d, want %d", version, currentSchemaVersion)
    }
    for table, want := range map[string]int{
        "channels": 1, "folders": 2, "files": 3,
        "dirents": 5, "file_revisions": 1,
    } {
        var got int
        if err := db.QueryRow("SELECT COUNT(*) FROM " + table).Scan(&got); err != nil {
            t.Fatal(err)
        }
        if got != want {
            t.Fatalf("%s count %d, want %d", table, got, want)
        }
    }
    var integrity string
    if err := db.QueryRow("PRAGMA integrity_check").Scan(&integrity); err != nil || integrity != "ok" {
        t.Fatalf("integrity %q: %v", integrity, err)
    }

    // Export the output of the authoritative Go read API for comparison with
    // the Rust implementation. This is synthetic test data, never real files.
    if oraclePath := os.Getenv("TDRIVE_COMPAT_FOLDER_LISTING_OUT"); oraclePath != "" {
        if !filepath.IsAbs(oraclePath) {
            t.Fatal("listing oracle path must be absolute")
        }
        if err := os.MkdirAll(filepath.Dir(oraclePath), 0o700); err != nil {
            t.Fatal(err)
        }
        if _, err := os.Lstat(oraclePath); err == nil {
            t.Fatal("refusing to replace existing listing oracle")
        } else if !os.IsNotExist(err) {
            t.Fatal(err)
        }
        cases := []struct {
            channelID int64
            parentID string
        }{
            {123456, ""},
            {123456, "d:docs"},
            {123456, "d:archive"},
            {123456, "d:missing"},
            {654321, ""},
        }
        var oracle strings.Builder
        for _, testCase := range cases {
            fmt.Fprintf(&oracle, "CASE\t%d\t%s\n", testCase.channelID, testCase.parentID)
            folders, files, err := ListFolderContents(db, testCase.channelID, testCase.parentID)
            if err != nil {
                t.Fatalf("Go folder listing oracle: %v", err)
            }
            for _, folder := range folders {
                fmt.Fprintf(&oracle, "DIR\t%s\t%s\t%s\t%d\n",
                    folder.ID, folder.Name, folder.ParentID, folder.Revision)
            }
            for _, file := range files {
                fmt.Fprintf(&oracle, "FILE\t%d\t%d\t%s\t%d\t%s\t%d\t%s\t%d\t%s\t%d\t%d\t%t\t%d\n",
                    file.MsgID, file.ContentMsgID, file.ContentHash, file.Revision,
                    file.UploadUUID, file.PartCount, file.Name, file.Size, file.ParentID,
                    file.UploadTime, file.UploaderID, file.Encrypted, file.PlaintextSize)
            }
        }
        if err := os.WriteFile(oraclePath, []byte(oracle.String()), 0o600); err != nil {
            t.Fatal(err)
        }
    }
}

// Length-prefix fields so distinct row contents can never produce the same
// concatenation before hashing. Matches Rust digest::feed exactly.
func goDigestFeed(h hash.Hash, value string) {
    var prefix [8]byte
    binary.BigEndian.PutUint64(prefix[:], uint64(len(value)))
    _, _ = h.Write(prefix[:])
    _, _ = h.Write([]byte(value))
}
