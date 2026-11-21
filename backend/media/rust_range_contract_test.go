package media

import (
    "bufio"
    "os"
    "strconv"
    "strings"
    "testing"
)

// Shared with crates/tdrive-media/tests/go_range.rs. Exercises the live Go
// parseRangeHeader rather than repeating its implementation in the fixture.
func TestRustMediaRangeFixtures(t *testing.T) {
    f, err := os.Open("../../tests/fixtures/media/range.tsv")
    if err != nil {
        t.Fatal(err)
    }
    defer f.Close()

    scanner := bufio.NewScanner(f)
    count := 0
    for scanner.Scan() {
        row := scanner.Text()
        if row == "" || strings.HasPrefix(row, "#") {
            continue
        }
        fields := strings.Split(row, "\t")
        if len(fields) != 7 {
            t.Fatalf("bad fixture: %q", row)
        }
        raw := fields[1]
        if raw == "<none>" {
            raw = ""
        }
        size, err := strconv.ParseInt(fields[2], 10, 64)
        if err != nil {
            t.Fatal(err)
        }
        wantStart, err := strconv.ParseInt(fields[3], 10, 64)
        if err != nil {
            t.Fatal(err)
        }
        wantEnd, err := strconv.ParseInt(fields[4], 10, 64)
        if err != nil {
            t.Fatal(err)
        }
        wantPartial, err := strconv.ParseBool(fields[5])
        if err != nil {
            t.Fatal(err)
        }
        wantOK, err := strconv.ParseBool(fields[6])
        if err != nil {
            t.Fatal(err)
        }
        start, end, partial, ok := parseRangeHeader(raw, size)
        if start != wantStart || end != wantEnd || partial != wantPartial || ok != wantOK {
            t.Fatalf("%s: got (%d,%d,%t,%t), want (%d,%d,%t,%t)",
                fields[0], start, end, partial, ok,
                wantStart, wantEnd, wantPartial, wantOK)
        }
        count++
    }
    if err := scanner.Err(); err != nil {
        t.Fatal(err)
    }
    if count < 18 {
        t.Fatalf("too few cross-language Range cases: %d", count)
    }
}
