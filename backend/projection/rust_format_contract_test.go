package projection

import (
    "bufio"
    "os"
    "strings"
    "testing"
)

// These explicit canonical headers are jointly checked against Go Format
// and Rust format_simple_wire, not against a handwaved Rust encoder only.
func TestRustSimpleFormatFixtures(t *testing.T) {
    f, err := os.Open("../../tests/fixtures/tdx1/formatted.tsv")
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
        parts := strings.SplitN(line, "\t", 2)
        if len(parts) != 2 {
            t.Fatalf("bad fixture %q", line)
        }
        op, err := Parse(parts[1])
        if err != nil {
            t.Fatalf("%s: parse error: %v", parts[0], err)
        }
        if formatted := Format(op); formatted != parts[1] {
            t.Fatalf("%s: Go Format=%q, want %q", parts[0], formatted, parts[1])
        }
        count++
    }
    if err := scanner.Err(); err != nil {
        t.Fatal(err)
    }
    if count < 14 {
        t.Fatalf("expected 14 or more canonical fixtures, got %d", count)
    }
}
