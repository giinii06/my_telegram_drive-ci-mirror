package crypto

import (
    "bytes"
    cryptorand "crypto/rand"
    "fmt"
    "os"
    "path/filepath"
    "strings"
    "testing"
)

// TestExportRustNameSealFixtures emits synthetic, deterministic Go ciphertext.
// Fixed nonce is TEST ONLY; production EncryptFileName always uses OS CSPRNG.
func TestExportRustNameSealFixtures(t *testing.T) {
    path := os.Getenv("TDRIVE_COMPAT_NAME_OUT")
    if path == "" {
        t.Skip("set fresh absolute filename fixture output path")
    }
    if !filepath.IsAbs(path) {
        t.Fatal("name fixture target must be absolute")
    }
    if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
        t.Fatal(err)
    }
    file, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
    if err != nil {
        t.Fatalf("fixture path must be new: %v", err)
    }
    defer file.Close()
    key := bytes.Repeat([]byte{11}, 32)
    cases := []struct{ id, name string }{
        {"ascii", "movie.mkv"},
        {"korean", "한글 파일 이름.mp4"},
        {"single", "a"},
        {"emoji", "사진🦋.jpg"},
        {"max", strings.Repeat("x", 4096)},
    }
    for _, tc := range cases {
        sealed, err := func() (string, error) {
            original := cryptorand.Reader
            cryptorand.Reader = bytes.NewReader(bytes.Repeat([]byte{0x19}, 24))
            defer func() { cryptorand.Reader = original }()
            return EncryptFileName(key, tc.name)
        }()
        if err != nil {
            t.Fatalf("Go seal case %s: %v", tc.id, err)
        }
        decoded, err := DecryptFileName(key, sealed)
        if err != nil || decoded != tc.name {
            t.Fatalf("Go name roundtrip case %s failed: %v", tc.id, err)
        }
        if _, err := fmt.Fprintf(file, "%s\t%s\n", tc.id, sealed); err != nil {
            t.Fatal(err)
        }
    }
    if err := file.Sync(); err != nil {
        t.Fatal(err)
    }
}
