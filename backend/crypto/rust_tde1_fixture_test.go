package crypto

import (
    "bytes"
    cryptorand "crypto/rand"
    "fmt"
    "os"
    "path/filepath"
    "testing"
)

// TestExportRustTDE1Fixtures only creates fresh synthetic ciphertext for the
// cross-language compatibility test. No user key, session or data is accessed.
// The output directory must not exist: accidental replacement is forbidden.
func TestExportRustTDE1Fixtures(t *testing.T) {
    root := os.Getenv("TDRIVE_COMPAT_TDE1_OUT")
    if root == "" {
        t.Skip("set TDRIVE_COMPAT_TDE1_OUT to a fresh absolute output directory")
    }
    if !filepath.IsAbs(root) {
        t.Fatal("TDE1 fixture destination must be absolute")
    }
    if err := os.MkdirAll(filepath.Dir(root), 0o700); err != nil {
        t.Fatal(err)
    }
    if err := os.Mkdir(root, 0o700); err != nil {
        t.Fatalf("fixture directory must not already exist: %v", err)
    }

    var key [32]byte
    for i := range key {
        key[i] = 0x42
    }
    for _, size := range []int{0, 1, 65535, 65536, 65537, 196625} {
        plain := make([]byte, size)
        for i := range plain {
            plain[i] = byte((i*31 + 7) % 251)
        }
        var ciphertext bytes.Buffer
        if err := EncryptStream(bytes.NewReader(plain), &ciphertext, key[:], int64(len(plain))); err != nil {
            t.Fatalf("EncryptStream(size=%d): %v", size, err)
        }
        if ciphertext.Len() != int(CiphertextSize(int64(size))) {
            t.Fatalf("ciphertext size mismatch for %d: %d", size, ciphertext.Len())
        }
        var roundtrip bytes.Buffer
        gotSize, err := DecryptStream(bytes.NewReader(ciphertext.Bytes()), &roundtrip, key[:])
        if err != nil || gotSize != int64(size) || !bytes.Equal(roundtrip.Bytes(), plain) {
            t.Fatalf("Go roundtrip(size=%d): gotSize=%d, err=%v", size, gotSize, err)
        }
        path := filepath.Join(root, fmt.Sprintf("tde1-%d.bin", size))
        f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
        if err != nil {
            t.Fatal(err)
        }
        _, writeErr := f.Write(ciphertext.Bytes())
        closeErr := f.Close()
        if writeErr != nil || closeErr != nil {
            t.Fatalf("writing synthetic TDE1 fixture: write=%v close=%v", writeErr, closeErr)
        }
    }

    // Exact Go ↔ Rust writer compatibility vectors. A deterministic entropy
    // source is ONLY used in this synthetic test; production encrypt always
    // uses the OS cryptographic source.
    for _, size := range []int{0, 1, 65535, 65536, 65537, 196625} {
        plain := make([]byte, size)
        for i := range plain {
            plain[i] = byte((i*31 + 7) % 251)
        }
        entropy := append(bytes.Repeat([]byte{0x11}, 16), bytes.Repeat([]byte{0x22}, 20)...)
        var expected bytes.Buffer
        encryptErr := func() error {
            original := cryptorand.Reader
            cryptorand.Reader = bytes.NewReader(entropy)
            defer func() { cryptorand.Reader = original }()
            return EncryptStream(bytes.NewReader(plain), &expected, key[:], int64(len(plain)))
        }()
        if encryptErr != nil {
            t.Fatalf("seeded Go EncryptStream(%d): %v", size, encryptErr)
        }
        if int64(expected.Len()) != CiphertextSize(int64(size)) {
            t.Fatalf("seeded writer size mismatch for %d", size)
        }
        target := filepath.Join(root, fmt.Sprintf("tde1-seeded-%d.bin", size))
        f, err := os.OpenFile(target, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
        if err != nil {
            t.Fatal(err)
        }
        _, writeErr := f.Write(expected.Bytes())
        closeErr := f.Close()
        if writeErr != nil || closeErr != nil {
            t.Fatalf("seeded TDE1 output: write=%v close=%v", writeErr, closeErr)
        }
    }
}
