package main

import (
    "bufio"
    "os"
    "strings"
    "testing"
)

// Read the same synthetic fixture as the Rust backup-core status contract.
// This test never contacts Telegram, runs uploads, or deletes local files.
func TestRustBackupJobVectors(t *testing.T) {
    file, err := os.Open("../../tests/fixtures/backup/upload-status.tsv")
    if err != nil {
        t.Fatal(err)
    }
    defer file.Close()
    scanner := bufio.NewScanner(file)
    count := 0
    for scanner.Scan() {
        line := scanner.Text()
        if line == "" || strings.HasPrefix(line, "#") {
            continue
        }
        fields := strings.Split(line, "\t")
        if len(fields) != 5 {
            t.Fatalf("bad fixture: %q", line)
        }
        job := uploadJob{}
        if fields[1] != "-" {
            for _, stage := range strings.Split(fields[1], ",") {
                status := uploadStatus{Stage: stage}
                if stage == "error" && fields[2] != "-" {
                    status.Error = fields[2]
                }
                job.Files = append(job.Files, status)
            }
        }
        gotDone, gotFailure := summarizeUpload(job)
        wantFailure := fields[4]
        if wantFailure == "-" {
            wantFailure = ""
        }
        if gotDone != (fields[3] == "true") || gotFailure != wantFailure {
            t.Fatalf("%s: summary = (%t,%q), want (%s,%q)",
                fields[0], gotDone, gotFailure, fields[3], wantFailure)
        }
        count++
    }
    if err := scanner.Err(); err != nil {
        t.Fatal(err)
    }
    if count < 11 {
        t.Fatalf("only %d shared backup fixtures", count)
    }
}
