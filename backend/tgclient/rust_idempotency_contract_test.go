package tgclient

import (
    "bufio"
    "os"
    "strconv"
    "strings"
    "testing"
)

// Cross-language golden vectors pin the actual stable random_id used by Go
// send receipts, so a Rust retry cannot duplicate a remote Telegram message.
func TestRustStableRandomIDVectors(t *testing.T) {
    f, err := os.Open("../../tests/fixtures/telegram/random-ids.tsv")
    if err != nil {
        t.Fatal(err)
    }
    defer f.Close()
    scanner := bufio.NewScanner(f)
    count := 0
    for scanner.Scan() {
        line := scanner.Text()
        if line == "" || strings.HasPrefix(line, "#") {
            continue
        }
        fields := strings.Split(line, "\t")
        if len(fields) != 3 {
            t.Fatalf("bad fixture %q", line)
        }
        want, err := strconv.ParseInt(fields[2], 10, 64)
        if err != nil {
            t.Fatal(err)
        }
        got, err := StableRandomID(fields[0], fields[1])
        if err != nil {
            t.Fatal(err)
        }
        if got != want {
            t.Fatalf("StableRandomID(%q,%q)=%d, want %d", fields[0], fields[1], got, want)
        }
        count++
    }
    if err := scanner.Err(); err != nil {
        t.Fatal(err)
    }
    if count < 5 {
        t.Fatalf("insufficient cross-language cases: %d", count)
    }
}
