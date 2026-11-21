package projection

import (
    "database/sql"
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
