package projection

import (
    "bufio"
    "errors"
    "os"
    "strings"
    "testing"
)

// TestRustTDX1Fixtures runs the same source-backed cases as the standalone
// Rust decoder. It is deliberately independent of live Telegram credentials.
// Unsupported rendition is a tracked Rust limitation: Go must still reject
// the malformed example; successful rendition fixtures are a later phase.
func TestRustTDX1Fixtures(t *testing.T) {
    file, err := os.Open("../../tests/fixtures/tdx1/cases.tsv")
    if err != nil {
        t.Fatalf("open TDX1 golden fixtures: %v", err)
    }
    defer file.Close()

    expectedErrors := map[string]error{
        "missing_header": ErrWireMissingHeader,
        "bad_version":    ErrWireBadVersion,
        "bad_op_type":    ErrWireBadOpType,
        "malformed":      ErrWireMalformed,
        "bad_object":     ErrWireBadObject,
        "bad_parent":     ErrWireBadParent,
    }

    scanner := bufio.NewScanner(file)
    scanner.Buffer(make([]byte, 4096), 1024*1024)
    count := 0
    for scanner.Scan() {
        row := scanner.Text()
        if row == "" || strings.HasPrefix(row, "#") {
            continue
        }
        fields := strings.SplitN(row, "\t", 3)
        if len(fields) != 3 {
            t.Fatalf("bad fixture row %q", row)
        }
        name, expected, raw := fields[0], fields[1], fields[2]
        t.Run(name, func(t *testing.T) {
            result, err := Parse(raw)
            if kind, ok := strings.CutPrefix(expected, "ok:"); ok {
                if err != nil {
                    t.Fatalf("Parse rejected expected %s: %v", kind, err)
                }
                if string(result.Type) != kind {
                    t.Fatalf("op type %q, want %q", result.Type, kind)
                }
                return
            }
            errorCode, ok := strings.CutPrefix(expected, "err:")
            if !ok {
                t.Fatalf("bad fixture expected outcome %q", expected)
            }
            if errorCode == "unsupported_rendition" {
                // The fixture has invalid rendition JSON and Go must reject it.
                // Rust currently rejects all renditions until JSON validation
                // has its own Go↔Rust positive and negative golden vectors.
                if err == nil {
                    t.Fatal("expected Go to reject malformed rendition")
                }
                return
            }
            want, ok := expectedErrors[errorCode]
            if !ok {
                t.Fatalf("unexpected error code %q", errorCode)
            }
            if !errors.Is(err, want) {
                t.Fatalf("Parse(%q): %v, want %v", raw, err, want)
            }
        })
        count++
    }
    if err := scanner.Err(); err != nil {
        t.Fatalf("read golden fixtures: %v", err)
    }
    if count < 40 {
        t.Fatalf("contract cases decreased: %d", count)
    }
}
